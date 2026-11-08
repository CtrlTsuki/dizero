//! On-disk plugin registry: discovery, install, integrity, permissions,
//! upgrade/rollback and uninstall.
//!
//! Installed plugins are copied into an app-owned directory
//! (`<app data>/plugins/<id>`), so later edits to the source folder have no
//! effect. A SHA-256 digest over every installed file is recorded at install
//! time and re-verified before each load; a mismatch blocks the plugin.

use super::host::{LoadRequest, PluginStorage};
use super::manifest::{MANIFEST_FILE, MAX_MANIFEST_BYTES, PluginKind, PluginManifest, ValidationIssue};
use super::permissions::Permission;
use crate::db::Database;
use crate::error::{CoreError, Result};
use crate::util::{hex, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ts_rs::TS;

pub const MAX_FILES: usize = 200;
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 5 * 1024 * 1024;
const MAX_DEPTH: usize = 5;
const ALLOWED_EXTENSIONS: &[&str] = &["js", "json", "md", "txt", "png", "svg"];
const ALLOWED_BARE_NAMES: &[&str] = &["LICENSE", "README", "NOTICE", "CHANGELOG"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PluginSource {
    Bundled,
    Local,
}

impl PluginSource {
    fn as_str(&self) -> &'static str {
        match self {
            PluginSource::Bundled => "bundled",
            PluginSource::Local => "local",
        }
    }
    fn parse(s: &str) -> Self {
        if s == "bundled" { PluginSource::Bundled } else { PluginSource::Local }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RegistryState {
    Disabled,
    Enabled,
    IntegrityFailed,
    DependencyMissing,
}

impl RegistryState {
    fn as_str(&self) -> &'static str {
        match self {
            RegistryState::Disabled => "disabled",
            RegistryState::Enabled => "enabled",
            RegistryState::IntegrityFailed => "integrity_failed",
            RegistryState::DependencyMissing => "dependency_missing",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "enabled" => RegistryState::Enabled,
            "integrity_failed" => RegistryState::IntegrityFailed,
            "dependency_missing" => RegistryState::DependencyMissing,
            _ => RegistryState::Disabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PermissionGrant {
    pub permission: Permission,
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub source: PluginSource,
    pub enabled: bool,
    pub state: RegistryState,
    pub permissions: Vec<PermissionGrant>,
    pub integrity: String,
    pub install_path: String,
    #[ts(type = "number")]
    pub installed_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    pub last_error: Option<String>,
    pub has_backup: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogEntry {
    pub folder: String,
    pub manifest: Option<PluginManifest>,
    pub issues: Vec<ValidationIssue>,
}

/// `(plugin id, human-readable reason)` for an enabled plugin that can't load.
pub type LoadProblem = (String, String);

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ValidationReport {
    pub manifest: Option<PluginManifest>,
    pub issues: Vec<ValidationIssue>,
    pub file_count: u32,
    #[ts(type = "number")]
    pub total_bytes: u64,
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

pub struct DbPluginStorage {
    db: Arc<Database>,
}

impl DbPluginStorage {
    pub fn new(db: Arc<Database>) -> Self {
        DbPluginStorage { db }
    }
}

impl PluginStorage for DbPluginStorage {
    fn get(&self, p: &str, k: &str) -> Result<Option<String>> {
        self.db.with(|c| {
            Ok(c.prepare_cached("SELECT value FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2")?
                .query_row(params![p, k], |r| r.get(0))
                .optional()?)
        })
    }
    fn set(&self, p: &str, k: &str, v: &str) -> Result<()> {
        self.db.with(|c| {
            c.prepare_cached(
                "INSERT INTO plugin_storage(plugin_id, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT(plugin_id, key) DO UPDATE SET value = excluded.value",
            )?
            .execute(params![p, k, v])?;
            Ok(())
        })
    }
    fn delete(&self, p: &str, k: &str) -> Result<()> {
        self.db.with(|c| {
            c.prepare_cached("DELETE FROM plugin_storage WHERE plugin_id = ?1 AND key = ?2")?.execute(params![p, k])?;
            Ok(())
        })
    }
    fn keys(&self, p: &str) -> Result<Vec<String>> {
        self.db.with(|c| {
            Ok(c.prepare_cached("SELECT key FROM plugin_storage WHERE plugin_id = ?1 ORDER BY key")?
                .query_map([p], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?)
        })
    }
    fn usage_bytes(&self, p: &str) -> Result<usize> {
        self.db.with(|c| {
            let n: i64 = c
                .prepare_cached("SELECT COALESCE(SUM(LENGTH(CAST(key AS BLOB)) + LENGTH(CAST(value AS BLOB))), 0) FROM plugin_storage WHERE plugin_id = ?1")?
                .query_row([p], |r| r.get(0))?;
            Ok(n.max(0) as usize)
        })
    }
}

// ---------------------------------------------------------------------------
// File helpers
// ---------------------------------------------------------------------------

struct FileEntry {
    rel: String,
    abs: PathBuf,
    size: u64,
}

fn is_allowed_file(name: &str) -> bool {
    if ALLOWED_BARE_NAMES.contains(&name) {
        return true;
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && ALLOWED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// Walk a plugin folder enforcing type, size, count, depth and symlink rules.
/// Dot-files and dot-directories (e.g. `.git`) are skipped.
fn collect_files(root: &Path) -> std::result::Result<Vec<FileEntry>, String> {
    let mut out = Vec::new();
    let mut total: u64 = 0;
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let meta = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() {
                return Err(format!("`{name}` is a symbolic link; links are not allowed in plugins"));
            }
            if meta.is_dir() {
                if depth + 1 > MAX_DEPTH {
                    return Err(format!("folder nesting deeper than {MAX_DEPTH} levels"));
                }
                stack.push((entry.path(), depth + 1));
                continue;
            }
            if !is_allowed_file(&name) {
                return Err(format!(
                    "`{name}` has a file type plugins may not contain (allowed: .{}, LICENSE/README)",
                    ALLOWED_EXTENSIONS.join(", .")
                ));
            }
            if meta.len() > MAX_FILE_BYTES {
                return Err(format!("`{name}` is larger than {} MiB", MAX_FILE_BYTES / 1024 / 1024));
            }
            total += meta.len();
            if total > MAX_TOTAL_BYTES {
                return Err(format!("plugin is larger than {} MiB in total", MAX_TOTAL_BYTES / 1024 / 1024));
            }
            let rel = entry
                .path()
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            out.push(FileEntry { rel, abs: entry.path(), size: meta.len() });
            if out.len() > MAX_FILES {
                return Err(format!("plugin contains more than {MAX_FILES} files"));
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

/// SHA-256 over every file's path, length and bytes, in sorted order.
fn digest_dir(root: &Path) -> std::result::Result<String, String> {
    let files = collect_files(root)?;
    let mut h = Sha256::new();
    for f in &files {
        let bytes = std::fs::read(&f.abs).map_err(|e| e.to_string())?;
        h.update(f.rel.as_bytes());
        h.update([0u8]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    Ok(hex(&h.finalize()))
}

fn read_manifest(dir: &Path) -> std::result::Result<PluginManifest, Vec<ValidationIssue>> {
    let path = dir.join(MANIFEST_FILE);
    let issue = |m: String| vec![ValidationIssue { field: "manifest".into(), message: m }];
    let meta = std::fs::metadata(&path).map_err(|_| issue(format!("{MANIFEST_FILE} not found")))?;
    if meta.len() as usize > MAX_MANIFEST_BYTES {
        return Err(issue(format!("{MANIFEST_FILE} is too large")));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| issue(format!("cannot read {MANIFEST_FILE}: {e}")))?;
    PluginManifest::parse(&text)
}

fn issues_to_error(issues: &[ValidationIssue]) -> CoreError {
    CoreError::invalid(issues.iter().map(|i| format!("{}: {}", i.field, i.message)).collect::<Vec<_>>().join("; "))
}

fn copy_files(files: &[FileEntry], dest: &Path) -> Result<()> {
    for f in files {
        let target = dest.join(&f.rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&f.abs, &target)?;
    }
    Ok(())
}

fn remove_dir_if_exists(p: &Path) -> Result<()> {
    if p.exists() {
        std::fs::remove_dir_all(p)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct PluginRegistry {
    db: Arc<Database>,
    root: PathBuf,
}

impl PluginRegistry {
    pub fn new(db: Arc<Database>, root: PathBuf) -> Self {
        PluginRegistry { db, root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn dir_for(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }
    fn backup_for(&self, id: &str) -> PathBuf {
        self.root.join(".backup").join(id)
    }

    /// Validate a folder without installing it (Developer Center).
    pub fn validate_dir(dir: &Path) -> ValidationReport {
        let mut issues = Vec::new();
        let manifest = match read_manifest(dir) {
            Ok(m) => Some(m),
            Err(mut i) => {
                issues.append(&mut i);
                None
            }
        };
        let (file_count, total_bytes) = match collect_files(dir) {
            Ok(files) => {
                if let Some(m) = &manifest {
                    if let Some(entry) = &m.entry {
                        if !files.iter().any(|f| &f.rel == entry) {
                            issues.push(ValidationIssue {
                                field: "entry".into(),
                                message: format!("`{entry}` does not exist"),
                            });
                        }
                    }
                }
                (files.len() as u32, files.iter().map(|f| f.size).sum())
            }
            Err(e) => {
                issues.push(ValidationIssue { field: "files".into(), message: e });
                (0, 0)
            }
        };
        ValidationReport { manifest: if issues.is_empty() { manifest } else { None }, issues, file_count, total_bytes }
    }

    /// List plugin folders under `dir` (the bundled catalog).
    pub fn scan_catalog(dir: &Path) -> Vec<CatalogEntry> {
        let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
        let mut out: Vec<CatalogEntry> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .map(|e| {
                let folder = e.file_name().to_string_lossy().to_string();
                match read_manifest(&e.path()) {
                    Ok(m) => CatalogEntry { folder, manifest: Some(m), issues: vec![] },
                    Err(issues) => CatalogEntry { folder, manifest: None, issues },
                }
            })
            .collect();
        out.sort_by(|a, b| a.folder.cmp(&b.folder));
        out
    }

    pub fn install_from_dir(&self, src: &Path, source: PluginSource) -> Result<InstalledPlugin> {
        let manifest = read_manifest(src).map_err(|i| issues_to_error(&i))?;
        let files = collect_files(src).map_err(CoreError::Invalid)?;
        if let Some(entry) = &manifest.entry {
            if !files.iter().any(|f| &f.rel == entry) {
                return Err(CoreError::invalid(format!("entry: `{entry}` does not exist in the plugin folder")));
            }
        }
        let id = manifest.id.clone();
        let existing = self.get(&id).ok();
        if let Some(prev) = &existing {
            let old = semver::Version::parse(&prev.manifest.version).ok();
            let new = semver::Version::parse(&manifest.version).ok();
            if let (Some(old), Some(new)) = (old, new) {
                if new < old {
                    return Err(CoreError::invalid(format!(
                        "refusing to downgrade {id} from {} to {}; use rollback instead",
                        prev.manifest.version, manifest.version
                    )));
                }
            }
        }

        // Stage → verify digest → swap into place.
        std::fs::create_dir_all(&self.root)?;
        let staging = self.root.join(".staging").join(format!("{id}-{}", now_ms()));
        remove_dir_if_exists(&staging)?;
        std::fs::create_dir_all(&staging)?;
        if let Err(e) = copy_files(&files, &staging) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        let integrity = digest_dir(&staging).map_err(CoreError::Invalid)?;
        let dest = self.dir_for(&id);
        if existing.is_some() && dest.exists() {
            let backup = self.backup_for(&id);
            remove_dir_if_exists(&backup)?;
            std::fs::create_dir_all(backup.parent().expect("backup has parent"))?;
            std::fs::rename(&dest, &backup)?;
        } else {
            remove_dir_if_exists(&dest)?;
        }
        std::fs::rename(&staging, &dest)?;

        let previously_requested: BTreeSet<Permission> =
            existing.as_ref().map(|p| p.manifest.permissions.iter().copied().collect()).unwrap_or_default();
        let previously_granted: BTreeSet<Permission> = existing
            .as_ref()
            .map(|p| p.permissions.iter().filter(|g| g.granted).map(|g| g.permission).collect())
            .unwrap_or_default();
        let adds_permissions = manifest.permissions.iter().any(|p| !previously_requested.contains(p));
        let enabled = existing.as_ref().is_some_and(|p| p.enabled) && !adds_permissions;
        let now = now_ms();
        let manifest_json = serde_json::to_string(&manifest)?;
        let install_path = dest.to_string_lossy().to_string();

        self.db.with_tx(|tx| {
            tx.execute(
                "INSERT INTO plugin_registry(id, version, source, install_path, manifest, integrity, enabled, state, last_error, installed_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?9)
                 ON CONFLICT(id) DO UPDATE SET version = excluded.version, source = excluded.source,
                    install_path = excluded.install_path, manifest = excluded.manifest, integrity = excluded.integrity,
                    enabled = excluded.enabled, state = excluded.state, last_error = NULL, updated_at = excluded.updated_at",
                params![
                    id,
                    manifest.version,
                    source.as_str(),
                    install_path,
                    manifest_json,
                    integrity,
                    enabled,
                    if enabled { RegistryState::Enabled.as_str() } else { RegistryState::Disabled.as_str() },
                    now
                ],
            )?;
            tx.execute("DELETE FROM plugin_permissions WHERE plugin_id = ?1", [&id])?;
            for p in &manifest.permissions {
                tx.execute(
                    "INSERT INTO plugin_permissions(plugin_id, permission, granted) VALUES (?1, ?2, ?3)",
                    params![id, p.key(), previously_granted.contains(p)],
                )?;
            }
            Ok(())
        })?;
        log::info!("installed plugin {id} {} ({})", manifest.version, source.as_str());
        self.get(&id)
    }

    pub fn uninstall(&self, id: &str) -> Result<()> {
        let p = self.get(id)?;
        remove_dir_if_exists(Path::new(&p.install_path))?;
        remove_dir_if_exists(&self.backup_for(id))?;
        self.db.with_tx(|tx| {
            tx.execute("DELETE FROM plugin_registry WHERE id = ?1", [id])?;
            tx.execute("DELETE FROM plugin_storage WHERE plugin_id = ?1", [id])?;
            Ok(())
        })?;
        log::info!("uninstalled plugin {id}");
        Ok(())
    }

    fn row(&self, id: &str) -> Result<InstalledPlugin> {
        type Row = (String, String, String, String, bool, String, Option<String>, i64, i64);
        let row: Option<Row> = self.db.with(|c| {
            Ok(c.prepare_cached(
                "SELECT source, install_path, manifest, integrity, enabled, state, last_error, installed_at, updated_at
                 FROM plugin_registry WHERE id = ?1",
            )?
            .query_row([id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
            })
            .optional()?)
        })?;
        let Some((source, install_path, manifest, integrity, enabled, state, last_error, installed_at, updated_at)) =
            row
        else {
            return Err(CoreError::NotFound(format!("plugin {id} is not installed")));
        };
        let manifest: PluginManifest = serde_json::from_str(&manifest)?;
        let grants: Vec<(String, bool)> = self.db.with(|c| {
            Ok(c.prepare_cached("SELECT permission, granted FROM plugin_permissions WHERE plugin_id = ?1")?
                .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?)
        })?;
        let permissions = manifest
            .permissions
            .iter()
            .map(|p| PermissionGrant { permission: *p, granted: grants.iter().any(|(k, g)| k == p.key() && *g) })
            .collect();
        Ok(InstalledPlugin {
            has_backup: self.backup_for(id).exists(),
            manifest,
            source: PluginSource::parse(&source),
            enabled,
            state: RegistryState::parse(&state),
            permissions,
            integrity,
            install_path,
            installed_at,
            updated_at,
            last_error,
        })
    }

    pub fn get(&self, id: &str) -> Result<InstalledPlugin> {
        self.row(id)
    }

    pub fn list(&self) -> Result<Vec<InstalledPlugin>> {
        let ids: Vec<String> = self.db.with(|c| {
            Ok(c.prepare_cached("SELECT id FROM plugin_registry ORDER BY id")?
                .query_map([], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?)
        })?;
        ids.iter().map(|id| self.row(id)).collect()
    }

    fn set_state(&self, id: &str, enabled: bool, state: RegistryState) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE plugin_registry SET enabled = ?2, state = ?3, updated_at = ?4 WHERE id = ?1",
                params![id, enabled, state.as_str(), now_ms()],
            )?;
            Ok(())
        })
    }

    /// Returns a description of the first unmet dependency, if any.
    fn unmet_dependency(&self, manifest: &PluginManifest) -> Option<String> {
        for (dep, req) in &manifest.dependencies {
            let Ok(req_parsed) = semver::VersionReq::parse(req) else {
                return Some(format!("{dep} {req} (invalid requirement)"));
            };
            match self.row(dep) {
                Err(_) => return Some(format!("requires {dep} {req}, which is not installed")),
                Ok(p) => {
                    if !p.enabled {
                        return Some(format!("requires {dep} {req}, which is disabled"));
                    }
                    let ok = semver::Version::parse(&p.manifest.version).is_ok_and(|v| req_parsed.matches(&v));
                    if !ok {
                        return Some(format!("requires {dep} {req}, but {} is installed", p.manifest.version));
                    }
                }
            }
        }
        None
    }

    /// The user reviewed the permission request: record their choices and enable.
    pub fn approve_and_enable(&self, id: &str, granted: &[Permission]) -> Result<InstalledPlugin> {
        let p = self.row(id)?;
        for g in granted {
            if !p.manifest.permissions.contains(g) {
                return Err(CoreError::invalid(format!("{} did not request `{}`", id, g.key())));
            }
        }
        if let Some(reason) = self.unmet_dependency(&p.manifest) {
            return Err(CoreError::invalid(reason));
        }
        if !self.verify_integrity(id)? {
            return Err(CoreError::invalid(format!("{id} failed its integrity check; reinstall it")));
        }
        self.db.with_tx(|tx| {
            for perm in &p.manifest.permissions {
                tx.execute(
                    "UPDATE plugin_permissions SET granted = ?3 WHERE plugin_id = ?1 AND permission = ?2",
                    params![id, perm.key(), granted.contains(perm)],
                )?;
            }
            Ok(())
        })?;
        self.set_state(id, true, RegistryState::Enabled)?;
        self.row(id)
    }

    pub fn disable(&self, id: &str) -> Result<InstalledPlugin> {
        self.row(id)?;
        self.set_state(id, false, RegistryState::Disabled)?;
        self.row(id)
    }

    pub fn set_permission(&self, id: &str, perm: Permission, granted: bool) -> Result<InstalledPlugin> {
        let p = self.row(id)?;
        if !p.manifest.permissions.contains(&perm) {
            return Err(CoreError::invalid(format!("{id} did not request `{}`", perm.key())));
        }
        self.db.with(|c| {
            c.execute(
                "UPDATE plugin_permissions SET granted = ?3 WHERE plugin_id = ?1 AND permission = ?2",
                params![id, perm.key(), granted],
            )?;
            Ok(())
        })?;
        self.row(id)
    }

    /// Recompute the installed files' digest and compare with the recorded one.
    /// A mismatch marks the plugin `IntegrityFailed`.
    pub fn verify_integrity(&self, id: &str) -> Result<bool> {
        let p = self.row(id)?;
        let ok = digest_dir(Path::new(&p.install_path)).map(|d| d == p.integrity).unwrap_or(false);
        if !ok {
            log::warn!("plugin {id} failed integrity verification");
            self.set_state(id, p.enabled, RegistryState::IntegrityFailed)?;
        } else if p.state == RegistryState::IntegrityFailed {
            self.set_state(id, p.enabled, if p.enabled { RegistryState::Enabled } else { RegistryState::Disabled })?;
        }
        Ok(ok)
    }

    /// Build runtime load requests for every enabled, verified script plugin.
    /// Problems (integrity, dependencies, unreadable code) are returned as
    /// `(plugin id, reason)` rather than aborting startup.
    pub fn load_requests(&self) -> Result<(Vec<LoadRequest>, Vec<LoadProblem>)> {
        let mut reqs = Vec::new();
        let mut problems = Vec::new();
        for p in self.list()? {
            let id = p.manifest.id.clone();
            match self.load_request(&id) {
                Ok(Some(r)) => reqs.push(r),
                Ok(None) => {}
                Err(e) => problems.push((id, e.to_string())),
            }
        }
        Ok((reqs, problems))
    }

    /// Load request for one plugin: `Ok(None)` if it is disabled or a theme,
    /// `Err` (with a user-facing reason) if it is enabled but cannot run.
    pub fn load_request(&self, id: &str) -> Result<Option<LoadRequest>> {
        let p = self.row(id)?;
        if !p.enabled || p.manifest.kind != PluginKind::Script {
            return Ok(None);
        }
        if !self.verify_integrity(id)? {
            return Err(CoreError::invalid("files changed since installation (integrity check failed)"));
        }
        if let Some(reason) = self.unmet_dependency(&p.manifest) {
            self.set_state(id, true, RegistryState::DependencyMissing)?;
            return Err(CoreError::invalid(reason));
        } else if p.state == RegistryState::DependencyMissing {
            self.set_state(id, true, RegistryState::Enabled)?;
        }
        let entry = p.manifest.entry.clone().unwrap_or_default();
        let source = std::fs::read_to_string(Path::new(&p.install_path).join(&entry))
            .map_err(|e| CoreError::invalid(format!("cannot read {entry}: {e}")))?;
        let granted = p.permissions.iter().filter(|g| g.granted).map(|g| g.permission).collect();
        Ok(Some(LoadRequest { manifest: p.manifest, source, granted }))
    }

    /// Swap the installed version with the backup kept from the last upgrade.
    pub fn rollback(&self, id: &str) -> Result<InstalledPlugin> {
        let current = self.row(id)?;
        let backup = self.backup_for(id);
        if !backup.exists() {
            return Err(CoreError::NotFound(format!("no previous version of {id} to roll back to")));
        }
        let manifest = read_manifest(&backup).map_err(|i| issues_to_error(&i))?;
        let integrity = digest_dir(&backup).map_err(CoreError::Invalid)?;
        let dest = self.dir_for(id);
        let swap = self.root.join(".staging").join(format!("{id}-swap-{}", now_ms()));
        std::fs::create_dir_all(swap.parent().expect("has parent"))?;
        std::fs::rename(&dest, &swap)?;
        std::fs::rename(&backup, &dest)?;
        std::fs::rename(&swap, &backup)?;
        let requested: BTreeSet<Permission> = manifest.permissions.iter().copied().collect();
        let manifest_json = serde_json::to_string(&manifest)?;
        self.db.with_tx(|tx| {
            tx.execute(
                "UPDATE plugin_registry SET version = ?2, manifest = ?3, integrity = ?4, updated_at = ?5, last_error = NULL WHERE id = ?1",
                params![id, manifest.version, manifest_json, integrity, now_ms()],
            )?;
            for g in &current.permissions {
                if !requested.contains(&g.permission) {
                    tx.execute(
                        "DELETE FROM plugin_permissions WHERE plugin_id = ?1 AND permission = ?2",
                        params![id, g.permission.key()],
                    )?;
                }
            }
            for p in &requested {
                tx.execute(
                    "INSERT OR IGNORE INTO plugin_permissions(plugin_id, permission, granted) VALUES (?1, ?2, 0)",
                    params![id, p.key()],
                )?;
            }
            Ok(())
        })?;
        log::info!("rolled back plugin {id} to {}", manifest.version);
        self.row(id)
    }

    pub fn set_last_error(&self, id: &str, error: Option<&str>) -> Result<()> {
        self.db.with(|c| {
            c.execute("UPDATE plugin_registry SET last_error = ?2 WHERE id = ?1", params![id, error])?;
            Ok(())
        })
    }
}
