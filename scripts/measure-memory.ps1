<#
.SYNOPSIS
  Reproducible idle-memory and startup measurement for DiZero on Windows.

.DESCRIPTION
  Launches the app, times launch -> visible window (the window is shown only
  after the UI's first paint), waits for it to settle, then samples the whole
  process tree (DiZero + every WebView2 child) several times.

  Reports per-process and total:
    * Working set       (Get-Process WorkingSet64)
    * Private bytes     (Get-Process PrivateMemorySize64 = commit charge)
  Totals are the median across samples. Processes are classified by their
  WebView2 --type flag. The app is closed with WM_CLOSE (graceful shutdown).

.EXAMPLE
  ./scripts/measure-memory.ps1 -Runs 3 -Label "release baseline"
#>
param(
  [string]$Exe = "$PSScriptRoot\..\target\release\dizero.exe",
  [int]$Runs = 3,
  [int]$SettleSeconds = 20,
  [int]$Samples = 10,
  [int]$IntervalMs = 1000,
  [string]$Label = "release",
  [string]$BrowserArgs = "",
  [string]$OutDir = "$PSScriptRoot\..\docs\benchmarks\raw"
)

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
if (Get-Process dizero -ErrorAction SilentlyContinue) { throw "Close running DiZero instances first." }

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace DZ {
  public static class Win {
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    // Visible top-level window with the exact title, owned by pid.
    public static IntPtr FindVisible(uint pid, string title) {
      IntPtr found = IntPtr.Zero;
      EnumWindows((h, l) => {
        uint owner; GetWindowThreadProcessId(h, out owner);
        if (owner != pid || !IsWindowVisible(h)) return true;
        var sb = new StringBuilder(256); GetWindowTextW(h, sb, 256);
        if (sb.ToString() == title) { found = h; return false; }
        return true;
      }, IntPtr.Zero);
      return found;
    }
  }
}
"@

# The visible "DiZero" window owned by $ProcId (shown only after the UI's
# first paint), or zero.
function Get-MainWindow([int]$ProcId) { [DZ.Win]::FindVisible([uint32]$ProcId, "DiZero") }

$LogFile = "$env:LOCALAPPDATA\com.dizero.desktop\logs\dizero.log"
function Get-UiReadyMs {
  if (-not (Test-Path $LogFile)) { return $null }
  $line = Get-Content $LogFile -Tail 50 | Where-Object { $_ -match "UI ready (\d+) ms" } | Select-Object -Last 1
  if ($line -match "UI ready (\d+) ms") { return [int]$Matches[1] } else { return $null }
}

# Private working set per PID (the value Task Manager's Memory column shows).
function Get-PrivateWs {
  $map = @{}
  Get-CimInstance Win32_PerfRawData_PerfProc_Process -Property IDProcess, WorkingSetPrivate | ForEach-Object { $map[[int]$_.IDProcess] = [double]$_.WorkingSetPrivate }
  return $map
}

function Get-Tree([int]$RootPid) {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, CommandLine
  $ids = [System.Collections.Generic.List[int]]::new(); $ids.Add($RootPid)
  for ($i = 0; $i -lt $ids.Count; $i++) {
    foreach ($p in $all) { if ($p.ParentProcessId -eq $ids[$i] -and $p.ProcessId -ne $ids[$i] -and -not $ids.Contains([int]$p.ProcessId)) { $ids.Add([int]$p.ProcessId) } }
  }
  foreach ($id in $ids) {
    $c = $all | Where-Object ProcessId -eq $id
    $role = if ($id -eq $RootPid) { "dizero (core)" }
            elseif ($c.CommandLine -match "--type=([\w-]+)") {
              $t = $Matches[1]
              if ($t -eq "utility" -and $c.CommandLine -match "--utility-sub-type=([\w.]+)") { "utility: " + ($Matches[1] -split "\.")[-1] } else { $t }
            } else { "webview2 browser" }
    [pscustomobject]@{ Pid = $id; Role = $role }
  }
}

function Median([double[]]$xs) { $s = $xs | Sort-Object; $n = $s.Count; if ($n % 2) { $s[[int][math]::Floor($n / 2)] } else { ($s[$n / 2 - 1] + $s[$n / 2]) / 2 } }

$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
$ram = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
$os = Get-CimInstance Win32_OperatingSystem
$wv = (Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue).pv

$results = @()
for ($run = 1; $run -le $Runs; $run++) {
  if ($BrowserArgs) { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $BrowserArgs } else { Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $Exe -PassThru
  $win = [IntPtr]::Zero
  while ($sw.Elapsed.TotalSeconds -lt 60 -and $win -eq [IntPtr]::Zero) {
    Start-Sleep -Milliseconds 10
    $win = Get-MainWindow $proc.Id
  }
  $visibleMs = [math]::Round($sw.Elapsed.TotalMilliseconds)
  Start-Sleep -Seconds $SettleSeconds
  $uiReadyMs = Get-UiReadyMs

  $tree = @(Get-Tree $proc.Id)
  $perPid = @{}
  $totalsWs = @(); $totalsPriv = @(); $totalsPws = @()
  for ($s = 0; $s -lt $Samples; $s++) {
    $ws = 0; $pv = 0; $pw = 0
    $pwsMap = Get-PrivateWs
    foreach ($t in $tree) {
      $gp = Get-Process -Id $t.Pid -ErrorAction SilentlyContinue
      if (-not $gp) { continue }
      $p1 = [double]($pwsMap[$t.Pid])
      $ws += $gp.WorkingSet64; $pv += $gp.PrivateMemorySize64; $pw += $p1
      if (-not $perPid[$t.Pid]) { $perPid[$t.Pid] = @{ Role = $t.Role; Ws = @(); Pv = @(); Pw = @() } }
      $perPid[$t.Pid].Ws += $gp.WorkingSet64; $perPid[$t.Pid].Pv += $gp.PrivateMemorySize64; $perPid[$t.Pid].Pw += $p1
    }
    $totalsWs += $ws; $totalsPriv += $pv; $totalsPws += $pw
    Start-Sleep -Milliseconds $IntervalMs
  }

  $procs = foreach ($k in $perPid.Keys) {
    [pscustomobject]@{
      Role = $perPid[$k].Role
      PrivateWorkingSetMiB = [math]::Round((Median $perPid[$k].Pw) / 1MB, 1)
      WorkingSetMiB = [math]::Round((Median $perPid[$k].Ws) / 1MB, 1)
      PrivateBytesMiB = [math]::Round((Median $perPid[$k].Pv) / 1MB, 1)
    }
  }
  $r = [pscustomobject]@{
    Run = $run
    LaunchToVisibleMs = $visibleMs
    UiReadyMs = $uiReadyMs
    Processes = $tree.Count
    TotalPrivateWorkingSetMiB = [math]::Round((Median $totalsPws) / 1MB, 1)
    TotalWorkingSetMiB = [math]::Round((Median $totalsWs) / 1MB, 1)
    TotalPrivateBytesMiB = [math]::Round((Median $totalsPriv) / 1MB, 1)
    PerProcess = @($procs | Sort-Object PrivateWorkingSetMiB -Descending)
  }
  $results += $r
  Write-Host ("run {0}: visible {1} ms (UI ready {2} ms), {3} processes | private WS {4} MiB | WS {5} MiB | private bytes {6} MiB" -f `
      $run, $r.LaunchToVisibleMs, $r.UiReadyMs, $r.Processes, $r.TotalPrivateWorkingSetMiB, $r.TotalWorkingSetMiB, $r.TotalPrivateBytesMiB)

  $h = Get-MainWindow $proc.Id
  if ($h -ne [IntPtr]::Zero) { [void][DZ.Win]::PostMessageW($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
  if (-not $proc.WaitForExit(15000)) { Write-Warning "graceful exit timed out; killing"; Stop-Process -Id $proc.Id -Force }
  Start-Sleep -Seconds 2
}
Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue

$report = [pscustomobject]@{
  Label = $Label
  Date = (Get-Date).ToString("s")
  Machine = [pscustomobject]@{ Cpu = $cpu; RamGiB = $ram; Os = "$($os.Caption) build $($os.BuildNumber)"; WebView2 = $wv }
  Method = "Launch->visible window; settle $SettleSeconds s; $Samples samples at $IntervalMs ms; medians; whole process tree"
  BrowserArgs = $BrowserArgs
  Exe = $Exe
  ExeSizeMiB = [math]::Round((Get-Item $Exe).Length / 1MB, 2)
  Runs = $results
  MedianLaunchToVisibleMs = Median ($results | ForEach-Object LaunchToVisibleMs)
  MedianPrivateWorkingSetMiB = Median ($results | ForEach-Object TotalPrivateWorkingSetMiB)
  MedianWorkingSetMiB = Median ($results | ForEach-Object TotalWorkingSetMiB)
  MedianPrivateBytesMiB = Median ($results | ForEach-Object TotalPrivateBytesMiB)
}
New-Item -ItemType Directory -Force $OutDir | Out-Null
$file = Join-Path $OutDir ("{0}-{1}.json" -f (Get-Date -Format "yyyyMMdd-HHmmss"), ($Label -replace '[^\w-]', '_'))
$report | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $file
Write-Host ""
Write-Host ("MEDIAN  visible {0} ms | private working set {1} MiB | working set {2} MiB | private bytes {3} MiB" -f `
    $report.MedianLaunchToVisibleMs, $report.MedianPrivateWorkingSetMiB, $report.MedianWorkingSetMiB, $report.MedianPrivateBytesMiB)
Write-Host "per-process (last run):"
$results[-1].PerProcess | Format-Table -AutoSize | Out-String | Write-Host
Write-Host "saved $file"
