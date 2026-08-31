//! Read-only local Skill trust evidence for the Multica execution bridge.
//!
//! A Skill discovered in a Codex directory is not trusted merely because it
//! exists or was installed by the plugin hub. The plugin hub's `verified` flag
//! is retained as source evidence, but it is not user trust. Everything
//! remains `review_required` until a dedicated review flow records stronger
//! evidence.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::plugin_hub::{InstallKind, PluginHubInstallRecord};
use crate::unified_tool_inventory::{
    UnifiedToolAsset, UnifiedToolInventory, UnifiedToolInventoryRoots, scan_unified_tool_inventory,
};

pub const TRUST_STATE_TRUSTED: &str = "trusted";
pub const TRUST_STATE_REVIEW_REQUIRED: &str = "review_required";
const TRUST_STORE_VERSION: u32 = 1;
const MAX_TRUST_ENTRIES: usize = 512;
const MAX_SKILL_ID_LENGTH: usize = 240;
const MAX_DIGEST_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedSkillTrust {
    trusted: bool,
    manifest_digest: String,
    reviewed_at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedSkillTrustFile {
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, PersistedSkillTrust>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSkillTrustEntry {
    pub id: String,
    pub installed: bool,
    pub enabled: bool,
    pub trusted: bool,
    pub trust_state: String,
    pub source_kind: String,
    pub manifest_digest: Option<String>,
}

impl LocalSkillTrustEntry {
    pub fn dispatch_allowed(&self) -> bool {
        self.installed && self.enabled && self.trusted
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalSkillTrustSnapshot {
    entries: BTreeMap<String, LocalSkillTrustEntry>,
    /// Stable diagnostic codes only.  Paths, command lines and parse details
    /// are deliberately kept out of the snapshot consumed by the renderer.
    pub diagnostics: Vec<String>,
}

impl LocalSkillTrustSnapshot {
    pub fn get(&self, id: &str) -> Option<&LocalSkillTrustEntry> {
        skill_id_aliases(id)
            .iter()
            .find_map(|alias| self.entries.get(alias))
    }

    pub fn entries(&self) -> impl Iterator<Item = &LocalSkillTrustEntry> {
        self.entries.values()
    }
}

/// Read local Codex Skill inventory and plugin-hub install records without
/// changing any file or invoking a CLI/network operation. If one source is
/// unavailable, the adapter fails closed and keeps any entries from the other
/// source review-only.
pub fn read_local_skill_trust_snapshot(
    roots: &UnifiedToolInventoryRoots,
) -> LocalSkillTrustSnapshot {
    read_local_skill_trust_snapshot_at(roots, &crate::paths::default_multica_skill_trust_path())
}

/// Read local inventory plus explicit CCP-owned review decisions.  A damaged
/// or unreadable trust file fails closed: the inventory remains visible, but
/// every Skill stays `review_required`.
pub fn read_local_skill_trust_snapshot_at(
    roots: &UnifiedToolInventoryRoots,
    trust_path: &Path,
) -> LocalSkillTrustSnapshot {
    let inventory = match scan_unified_tool_inventory(roots) {
        Ok(inventory) => inventory,
        Err(_) => {
            return LocalSkillTrustSnapshot {
                entries: BTreeMap::new(),
                diagnostics: vec!["skill_inventory_unavailable".to_string()],
            };
        }
    };
    let (records, records_diagnostic) = match crate::plugin_hub::load_installed_records() {
        Ok(records) => (records, None),
        Err(_) => (
            BTreeMap::new(),
            Some("plugin_hub_records_unavailable".to_string()),
        ),
    };
    let (decisions, trust_diagnostic) = match load_trust_decisions(trust_path) {
        Ok(decisions) => (decisions, None),
        Err(_) => (
            BTreeMap::new(),
            Some("skill_trust_store_invalid".to_string()),
        ),
    };
    let mut snapshot =
        build_local_skill_trust_snapshot_with_decisions(&inventory, &records, roots, &decisions);
    if let Some(diagnostic) = records_diagnostic {
        snapshot.diagnostics.push(diagnostic);
    }
    if let Some(diagnostic) = trust_diagnostic {
        snapshot.diagnostics.push(diagnostic);
    }
    snapshot
}

/// Build a trust snapshot from already-read local state.  Keeping this pure
/// makes the evidence rules testable without touching the user's home or CCP
/// state directory.
pub fn build_local_skill_trust_snapshot(
    inventory: &UnifiedToolInventory,
    installed_records: &BTreeMap<String, PluginHubInstallRecord>,
    roots: &UnifiedToolInventoryRoots,
) -> LocalSkillTrustSnapshot {
    build_local_skill_trust_snapshot_with_decisions(
        inventory,
        installed_records,
        roots,
        &BTreeMap::new(),
    )
}

/// Pure projection used by both runtime reads and tests.  A persisted trust
/// decision is valid only for the exact reviewed manifest digest; editing a
/// `SKILL.md` automatically returns the Skill to review-required.
fn build_local_skill_trust_snapshot_with_decisions(
    inventory: &UnifiedToolInventory,
    installed_records: &BTreeMap<String, PluginHubInstallRecord>,
    roots: &UnifiedToolInventoryRoots,
    decisions: &BTreeMap<String, PersistedSkillTrust>,
) -> LocalSkillTrustSnapshot {
    let mut entries = BTreeMap::new();
    for asset in inventory
        .assets
        .iter()
        .filter(|asset| asset.kind == "skill")
    {
        let state = &asset.codex;
        let installed = state.available;
        let enabled = state.enabled;
        let manifest_digest = skill_manifest_digest(state);
        // `verified` proves that the plugin hub completed its managed install,
        // but it does not prove that a user reviewed/trusted the Skill. Keep
        // the source distinction visible while leaving dispatch closed until
        // an explicit Skill review flow exists.
        let verified_managed_install =
            installed && enabled && verified_managed_skill_path(asset, installed_records, roots);
        let source_kind = if verified_managed_install {
            "plugin_hub_verified_managed_skill_bundle"
        } else if installed {
            "codex_local_skill"
        } else {
            "unknown"
        };
        let decision = decisions.get(&skill_key(&asset.id));
        let explicitly_trusted = decision.is_some_and(|decision| {
            decision.trusted
                && manifest_digest.is_some()
                && manifest_digest.as_deref() == Some(decision.manifest_digest.as_str())
        });
        let entry = LocalSkillTrustEntry {
            id: asset.id.clone(),
            installed,
            enabled,
            trusted: explicitly_trusted,
            trust_state: if explicitly_trusted {
                TRUST_STATE_TRUSTED.to_string()
            } else {
                TRUST_STATE_REVIEW_REQUIRED.to_string()
            },
            source_kind: if explicitly_trusted {
                "explicit_review".to_string()
            } else {
                source_kind.to_string()
            },
            manifest_digest,
        };
        entries.insert(skill_key(&entry.id), entry);
    }

    LocalSkillTrustSnapshot {
        entries,
        diagnostics: inventory
            .diagnostics
            .iter()
            .map(|_| "skill_inventory_diagnostic".to_string())
            .collect(),
    }
}

/// Record or revoke a user's explicit review of one installed Codex Skill.
/// This function only writes CCP's private trust record; it never installs,
/// enables, executes, or modifies the Skill itself.
pub fn review_local_skill(
    id: &str,
    trusted: bool,
    manifest_digest: Option<&str>,
) -> anyhow::Result<Value> {
    review_local_skill_at(
        &UnifiedToolInventoryRoots::default(),
        &crate::paths::default_multica_skill_trust_path(),
        id,
        trusted,
        manifest_digest,
    )
}

/// Testable form of [`review_local_skill`] with explicit inventory roots and
/// trust path.  The path is supplied by Core only; renderer input is never
/// interpreted as a filesystem location.
pub fn review_local_skill_at(
    roots: &UnifiedToolInventoryRoots,
    trust_path: &Path,
    id: &str,
    trusted: bool,
    manifest_digest: Option<&str>,
) -> anyhow::Result<Value> {
    validate_skill_id(id)?;
    let inventory =
        scan_unified_tool_inventory(roots).map_err(|_| anyhow!("skill_inventory_unavailable"))?;
    let asset = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "skill" && skill_id_matches(&asset.id, id))
        .ok_or_else(|| anyhow!("skill_unknown"))?;
    if !asset.codex.available {
        bail!("skill_not_installed");
    }
    let current_digest =
        skill_manifest_digest(&asset.codex).ok_or_else(|| anyhow!("skill_manifest_unavailable"))?;
    if let Some(expected) = manifest_digest {
        // Compare first so an old caller pin is reported as a conflict rather
        // than being confused with a malformed persisted record.
        if expected != current_digest {
            bail!("skill_manifest_conflict");
        }
        validate_digest(expected)?;
    }

    // A malformed trust file must never be treated as an empty file: doing so
    // would silently discard prior review decisions when this operation saves
    // the new one. Fail closed and leave the damaged file untouched.
    let mut decisions = load_trust_decisions(trust_path)?;
    let key = skill_key(&asset.id);
    if trusted {
        decisions.insert(
            key,
            PersistedSkillTrust {
                trusted: true,
                manifest_digest: current_digest.clone(),
                reviewed_at_ms: now_ms(),
            },
        );
    } else {
        decisions.remove(&key);
    }
    save_trust_decisions(trust_path, &decisions)?;
    Ok(serde_json::json!({
        "status": "ok",
        "id": asset.id,
        "trusted": trusted,
        "trust_state": if trusted { TRUST_STATE_TRUSTED } else { TRUST_STATE_REVIEW_REQUIRED },
        "dispatch_allowed": trusted && asset.codex.enabled,
        "manifest_digest": current_digest,
    }))
}

fn load_trust_decisions(path: &Path) -> anyhow::Result<BTreeMap<String, PersistedSkillTrust>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(_) => bail!("skill_trust_store_read_failed"),
    };
    if bytes.len() > 256 * 1024 {
        bail!("skill_trust_store_too_large");
    }
    let file: PersistedSkillTrustFile =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("skill_trust_store_invalid"))?;
    if file.version != TRUST_STORE_VERSION || file.entries.len() > MAX_TRUST_ENTRIES {
        bail!("skill_trust_store_invalid");
    }
    let mut decisions = BTreeMap::new();
    for (id, decision) in file.entries {
        validate_skill_id(&id)?;
        validate_digest(&decision.manifest_digest)?;
        decisions.insert(skill_key(&id), decision);
    }
    Ok(decisions)
}

fn save_trust_decisions(
    path: &Path,
    decisions: &BTreeMap<String, PersistedSkillTrust>,
) -> anyhow::Result<()> {
    if decisions.len() > MAX_TRUST_ENTRIES {
        bail!("skill_trust_store_too_large");
    }
    let file = PersistedSkillTrustFile {
        version: TRUST_STORE_VERSION,
        entries: decisions.clone(),
    };
    let bytes =
        serde_json::to_vec_pretty(&file).map_err(|_| anyhow!("skill_trust_store_invalid"))?;
    crate::settings::atomic_write(path, &bytes)
        .map_err(|_| anyhow!("skill_trust_store_write_failed"))
}

fn validate_skill_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty()
        || id.len() > MAX_SKILL_ID_LENGTH
        || !id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'@')
        })
    {
        bail!("skill_id_invalid");
    }
    Ok(())
}

fn validate_digest(digest: &str) -> anyhow::Result<()> {
    let valid = if let Some(hex) = digest.strip_prefix("sha256:") {
        // Digests emitted by `skill_manifest_digest` carry their algorithm
        // prefix. Keep the shape strict so a review cannot pin arbitrary text.
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        !digest.is_empty()
            && digest.len() <= MAX_DIGEST_LENGTH
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || matches!(byte, b':' | b'-' | b'_'))
    };
    if !valid {
        bail!("skill_manifest_digest_invalid");
    }
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn skill_manifest_digest(
    state: &crate::unified_tool_inventory::UnifiedToolAppState,
) -> Option<String> {
    let source_path = Path::new(state.source_path.trim());
    if state.source_path.trim().is_empty() {
        return None;
    }
    let metadata = std::fs::metadata(source_path.join("SKILL.md")).ok()?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 {
        return None;
    }
    let bytes = std::fs::read(source_path.join("SKILL.md")).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Some(format!("sha256:{:x}", hasher.finalize()))
}

fn verified_managed_skill_path(
    asset: &UnifiedToolAsset,
    installed_records: &BTreeMap<String, PluginHubInstallRecord>,
    roots: &UnifiedToolInventoryRoots,
) -> bool {
    let source_path = Path::new(asset.codex.source_path.trim());
    if asset.codex.source_path.trim().is_empty()
        || !source_path.join("SKILL.md").is_file()
        || !path_is_within(source_path, &roots.codex_home.join("skills"))
    {
        return false;
    }
    installed_records.values().any(|record| {
        record.verified
            && record.install_kind == InstallKind::ManagedSkillBundle
            && record.managed_paths.iter().any(|managed_path| {
                let managed_path = PathBuf::from(managed_path);
                paths_equal(&managed_path, source_path)
            })
    })
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = path_key(path);
    let root = path_key(root);
    path == root || path.starts_with(&(root + "/"))
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    path_key(left) == path_key(right)
}

fn path_key(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut value = absolute.to_string_lossy().replace('\\', "/");
    while value.ends_with('/') && value.len() > 1 {
        value.pop();
    }
    if cfg!(windows) {
        value = value.to_ascii_lowercase();
    }
    value
}

fn skill_key(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

/// Runtime inventory IDs are namespaced (`codex:...`), while the existing
/// local inventory stores the normalized Skill name without that namespace.
/// Only the two known Skill namespaces are collapsed; arbitrary prefixes are
/// left untouched so unrelated IDs cannot accidentally share trust state.
fn skill_id_aliases(id: &str) -> Vec<String> {
    let key = skill_key(id);
    let mut aliases = vec![key.clone()];
    for prefix in ["codex:", "skill:"] {
        if let Some(unprefixed) = key.strip_prefix(prefix)
            && !unprefixed.is_empty()
            && !aliases.iter().any(|alias| alias == unprefixed)
        {
            aliases.push(unprefixed.to_string());
        }
    }
    aliases
}

fn skill_id_matches(left: &str, right: &str) -> bool {
    let left_aliases = skill_id_aliases(left);
    let right_aliases = skill_id_aliases(right);
    left_aliases
        .iter()
        .any(|alias| right_aliases.iter().any(|candidate| alias == candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified_tool_inventory::{UnifiedToolAppState, UnifiedToolInventoryCounts};

    fn roots(dir: &Path) -> UnifiedToolInventoryRoots {
        UnifiedToolInventoryRoots {
            codex_home: dir.join("codex"),
            claude_home: dir.join("claude"),
            claude_config_paths: Vec::new(),
        }
    }

    fn skill_asset(root: &Path, id: &str, enabled: bool) -> UnifiedToolAsset {
        let source_path = root.join("skills").join(id);
        std::fs::create_dir_all(&source_path).unwrap();
        std::fs::write(source_path.join("SKILL.md"), "# local skill").unwrap();
        UnifiedToolAsset {
            id: id.to_string(),
            kind: "skill".to_string(),
            title: id.to_string(),
            summary: "summary".to_string(),
            source: source_path.to_string_lossy().to_string(),
            codex: UnifiedToolAppState {
                enabled,
                available: true,
                toggle_supported: true,
                source_path: source_path.to_string_lossy().to_string(),
                ..UnifiedToolAppState::default()
            },
            ..UnifiedToolAsset::default()
        }
    }

    fn record(path: &Path, verified: bool, install_kind: InstallKind) -> PluginHubInstallRecord {
        PluginHubInstallRecord {
            id: "managed-skills".to_string(),
            name: "Managed Skills".to_string(),
            install_kind,
            installed_at: "1".to_string(),
            command: Vec::new(),
            source_url: "https://example.test".to_string(),
            backup_path: None,
            managed_paths: vec![path.to_string_lossy().to_string()],
            verified,
        }
    }

    fn inventory(asset: UnifiedToolAsset) -> UnifiedToolInventory {
        UnifiedToolInventory {
            assets: vec![asset],
            counts: UnifiedToolInventoryCounts::default(),
            scanned_sources: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn verified_managed_active_skill_stays_review_required_without_user_trust() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let asset = skill_asset(&root.codex_home, "build", true);
        let source = PathBuf::from(&asset.codex.source_path);
        let records = BTreeMap::from([(
            "managed".to_string(),
            record(&source, true, InstallKind::ManagedSkillBundle),
        )]);

        let snapshot = build_local_skill_trust_snapshot(&inventory(asset), &records, &root);
        let entry = snapshot.get("BUILD").unwrap();
        assert_eq!(entry.trust_state, TRUST_STATE_REVIEW_REQUIRED);
        assert!(!entry.dispatch_allowed());
        assert_eq!(
            entry.source_kind,
            "plugin_hub_verified_managed_skill_bundle"
        );
        assert!(
            entry
                .manifest_digest
                .as_deref()
                .is_some_and(|value| value.starts_with("sha256:"))
        );
    }

    #[test]
    fn dispatch_gate_requires_explicit_trusted_state() {
        let entry = LocalSkillTrustEntry {
            id: "reviewed".to_string(),
            installed: true,
            enabled: true,
            trusted: true,
            trust_state: TRUST_STATE_TRUSTED.to_string(),
            source_kind: "explicit_review".to_string(),
            manifest_digest: Some("sha256:abc".to_string()),
        };
        assert!(entry.dispatch_allowed());

        let mut disabled = entry.clone();
        disabled.enabled = false;
        assert!(!disabled.dispatch_allowed());
    }

    #[test]
    fn unverified_disabled_or_wrong_kind_skill_stays_review_required() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let active_asset = skill_asset(&root.codex_home, "active", true);
        let active_path = PathBuf::from(&active_asset.codex.source_path);
        let disabled_asset = skill_asset(&root.codex_home, "disabled", false);
        let disabled_path = PathBuf::from(&disabled_asset.codex.source_path);
        let wrong_kind_asset = skill_asset(&root.codex_home, "wrong-kind", true);
        let wrong_kind_path = PathBuf::from(&wrong_kind_asset.codex.source_path);
        let records = BTreeMap::from([
            (
                "active".to_string(),
                record(&active_path, false, InstallKind::ManagedSkillBundle),
            ),
            (
                "disabled".to_string(),
                record(&disabled_path, true, InstallKind::ManagedSkillBundle),
            ),
            (
                "wrong-kind".to_string(),
                record(&wrong_kind_path, true, InstallKind::SkillBundle),
            ),
        ]);
        let mut combined = inventory(active_asset);
        combined.assets.push(disabled_asset);
        combined.assets.push(wrong_kind_asset);

        let snapshot = build_local_skill_trust_snapshot(&combined, &records, &root);
        for id in ["active", "disabled", "wrong-kind"] {
            let entry = snapshot.get(id).unwrap();
            assert_eq!(entry.trust_state, TRUST_STATE_REVIEW_REQUIRED);
            assert!(!entry.dispatch_allowed());
        }
    }

    #[test]
    fn managed_path_must_match_active_codex_skill_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let asset = skill_asset(&root.codex_home, "build", true);
        let broader = root.codex_home.join("skills");
        let records = BTreeMap::from([(
            "managed".to_string(),
            record(&broader, true, InstallKind::ManagedSkillBundle),
        )]);

        let snapshot = build_local_skill_trust_snapshot(&inventory(asset), &records, &root);
        assert_eq!(
            snapshot.get("build").unwrap().trust_state,
            TRUST_STATE_REVIEW_REQUIRED
        );
    }

    #[test]
    fn runtime_namespace_alias_resolves_local_skill_trust() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let asset = skill_asset(&root.codex_home, "review-helper", true);
        let snapshot = build_local_skill_trust_snapshot(&inventory(asset), &BTreeMap::new(), &root);
        assert_eq!(
            snapshot
                .get("codex:review-helper")
                .map(|entry| entry.id.as_str()),
            Some("review-helper")
        );
        assert_eq!(
            snapshot
                .get("skill:review-helper")
                .map(|entry| entry.id.as_str()),
            Some("review-helper")
        );
        assert!(snapshot.get("other:review-helper").is_none());
    }

    #[test]
    fn review_accepts_codex_runtime_namespace_but_persists_local_id() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        skill_asset(&root.codex_home, "review-helper", true);
        let trust_path = dir.path().join("skill-trust.json");
        let inventory = scan_unified_tool_inventory(&root).unwrap();
        let asset = inventory
            .assets
            .iter()
            .find(|asset| asset.kind == "skill")
            .unwrap();
        let digest = skill_manifest_digest(&asset.codex).unwrap();
        let result = review_local_skill_at(
            &root,
            &trust_path,
            "codex:review-helper",
            true,
            Some(&digest),
        )
        .unwrap();
        assert_eq!(result["id"], "review-helper");
        assert!(
            read_local_skill_trust_snapshot_at(&root, &trust_path)
                .get("codex:review-helper")
                .is_some_and(LocalSkillTrustEntry::dispatch_allowed)
        );
    }

    #[test]
    fn explicit_review_is_persisted_and_revoked_without_touching_skill_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let asset = skill_asset(&root.codex_home, "review-me", true);
        let skill_path = PathBuf::from(&asset.codex.source_path).join("SKILL.md");
        let trust_path = dir.path().join("skill-trust.json");
        let digest = skill_manifest_digest(&asset.codex).unwrap();

        let trusted =
            review_local_skill_at(&root, &trust_path, "review-me", true, Some(&digest)).unwrap();
        assert_eq!(trusted["trust_state"], "trusted");
        assert!(trusted["dispatch_allowed"].as_bool().unwrap());
        let snapshot = read_local_skill_trust_snapshot_at(&root, &trust_path);
        assert!(snapshot.get("review-me").unwrap().dispatch_allowed());
        assert_eq!(
            std::fs::read_to_string(&skill_path).unwrap(),
            "# local skill"
        );

        let revoked =
            review_local_skill_at(&root, &trust_path, "review-me", false, Some(&digest)).unwrap();
        assert_eq!(revoked["trust_state"], "review_required");
        assert!(
            !read_local_skill_trust_snapshot_at(&root, &trust_path)
                .get("review-me")
                .unwrap()
                .dispatch_allowed()
        );
        assert!(
            !serde_json::to_string(&trusted)
                .unwrap()
                .contains("SKILL.md")
        );
    }

    #[test]
    fn review_rejects_stale_manifest_digest() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        skill_asset(&root.codex_home, "review-me", true);
        let error = review_local_skill_at(
            &root,
            &dir.path().join("skill-trust.json"),
            "review-me",
            true,
            Some("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "skill_manifest_conflict");
    }

    #[test]
    fn review_rejects_damaged_trust_file_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = roots(dir.path());
        let asset = skill_asset(&root.codex_home, "review-me", true);
        let digest = skill_manifest_digest(&asset.codex).unwrap();
        let trust_path = dir.path().join("skill-trust.json");
        let damaged = b"{not-json";
        std::fs::write(&trust_path, damaged).unwrap();

        let error = review_local_skill_at(&root, &trust_path, "review-me", true, Some(&digest))
            .unwrap_err();
        assert_eq!(error.to_string(), "skill_trust_store_invalid");
        assert_eq!(std::fs::read(&trust_path).unwrap(), damaged);
    }
}
