# Prints total CPU seconds used by dizero.exe and all of its descendant
# processes (WebView2). Used by tests/e2e/scroll-bench.mjs.
$root = (Get-Process dizero -ErrorAction Stop | Select-Object -First 1).Id
$all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
$ids = [System.Collections.Generic.List[int]]::new()
$ids.Add($root)
for ($i = 0; $i -lt $ids.Count; $i++) {
  foreach ($p in $all) {
    if ($p.ParentProcessId -eq $ids[$i] -and -not $ids.Contains([int]$p.ProcessId)) { $ids.Add([int]$p.ProcessId) }
  }
}
$total = 0.0
foreach ($id in $ids) {
  $g = Get-Process -Id $id -ErrorAction SilentlyContinue
  if ($g) { $total += $g.TotalProcessorTime.TotalSeconds }
}
"{0:F4}" -f $total
