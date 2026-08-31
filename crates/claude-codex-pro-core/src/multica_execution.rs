//! Deterministic execution and Skill policy primitives for the Multica bridge.
//!
//! The module intentionally contains no HTTP, shell, provider, or renderer
//! code.  A production execution adapter supplies the authoritative Codex
//! inventory and records the returned loaded set around these pure checks.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_SKILL_REFS: usize = 64;
const MAX_SKILL_ID_LENGTH: usize = 240;
const MAX_DIGEST_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillBindingScope {
    Task,
    Agent,
}

impl SkillBindingScope {
    /// Canonical wire/storage spelling used by bridge payloads and binding
    /// identity hashes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Agent => "agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillReference {
    pub id: String,
    #[serde(default)]
    pub manifest_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillInventoryEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    pub installed: bool,
    pub trusted: bool,
    pub compatible: bool,
    #[serde(default)]
    pub manifest_digest: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindings {
    #[serde(default)]
    pub task: Vec<SkillReference>,
    #[serde(default)]
    pub agent: Vec<SkillReference>,
}

/// Renderer-facing selection.  Inventory, trust and Runtime capabilities are
/// deliberately absent: Core resolves those from authoritative local/runtime
/// state instead of accepting a renderer self-report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillBindingSelection {
    #[serde(default)]
    pub bindings: SkillBindings,
}

impl SkillBindingSelection {
    pub fn validate(&self) -> anyhow::Result<()> {
        let total = self.bindings.task.len() + self.bindings.agent.len();
        if total > MAX_SKILL_REFS {
            bail!("skill_refs_too_large");
        }
        for reference in self.bindings.task.iter().chain(self.bindings.agent.iter()) {
            validate_reference(reference)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillResolutionRequest {
    pub bindings: SkillBindings,
    pub runtime_capabilities: Vec<String>,
    pub inventory: Vec<SkillInventoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillResolutionAudit {
    pub requested_skill_refs: Vec<SkillReference>,
    pub resolved_skill_refs: Vec<SkillReference>,
    pub resolved_manifest_digest: String,
    pub protocol: String,
    pub resolution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillLoadAudit {
    pub requested_skill_refs: Vec<SkillReference>,
    pub resolved_skill_refs: Vec<SkillReference>,
    pub resolved_manifest_digest: String,
    pub runtime_loaded_skill_refs: Vec<SkillReference>,
    pub protocol: String,
    pub resolution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_error_code: Option<String>,
}

/// The narrow payload that a Codex Runtime adapter may pass to its native
/// Skill-loading request. It contains stable references and the immutable
/// manifest digest only; no Skill body, path, command, or credential is
/// carried across the execution boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSkillExecutionRequest {
    pub protocol: String,
    pub skill_refs: Vec<SkillReference>,
    pub manifest_digest: String,
}

impl SkillResolutionAudit {
    pub fn execution_request(&self) -> anyhow::Result<CodexSkillExecutionRequest> {
        if self.resolution_status != "resolved" {
            bail!("skill_resolution_not_ready");
        }
        Ok(CodexSkillExecutionRequest {
            protocol: self.protocol.clone(),
            skill_refs: self.resolved_skill_refs.clone(),
            manifest_digest: self.resolved_manifest_digest.clone(),
        })
    }
}

pub fn resolve_skill_bindings(
    request: &SkillResolutionRequest,
) -> anyhow::Result<SkillResolutionAudit> {
    validate_capabilities(&request.runtime_capabilities)?;
    let requested = effective_skill_refs(&request.bindings)?;
    let inventory = inventory_map(&request.inventory)?;
    let protocol = skill_protocol(&request.runtime_capabilities).to_string();
    let mut resolved = Vec::with_capacity(requested.len());
    for reference in &requested {
        let entry = inventory
            .get(&reference.id)
            .ok_or_else(|| anyhow!("skill_unknown"))?;
        if !entry.installed {
            bail!("skill_not_installed");
        }
        if !entry.trusted {
            bail!("skill_not_trusted");
        }
        if !entry.compatible {
            bail!("skill_incompatible");
        }
        if let Some(expected) = reference.manifest_digest.as_deref() {
            if entry.manifest_digest.as_deref() != Some(expected) {
                bail!("skill_manifest_conflict");
            }
        }
        let mut canonical = reference.clone();
        if canonical.manifest_digest.is_none() {
            canonical.manifest_digest = entry.manifest_digest.clone();
        }
        resolved.push(canonical);
    }
    let resolved_manifest_digest = manifest_digest(&resolved, &inventory);
    Ok(SkillResolutionAudit {
        requested_skill_refs: requested.clone(),
        resolved_skill_refs: resolved,
        resolved_manifest_digest,
        protocol,
        resolution_status: "resolved".to_string(),
        resolution_error_code: None,
    })
}

pub fn record_runtime_loaded(
    resolved: &SkillResolutionAudit,
    loaded: Vec<SkillReference>,
) -> anyhow::Result<SkillLoadAudit> {
    validate_refs(&resolved.resolved_skill_refs)?;
    validate_refs(&loaded)?;
    let expected_ids = resolved
        .resolved_skill_refs
        .iter()
        .map(|reference| reference.id.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let observed_ids = loaded
        .iter()
        .map(|reference| reference.id.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let expected_digests = resolved
        .resolved_skill_refs
        .iter()
        .map(|reference| {
            (
                reference.id.to_ascii_lowercase(),
                reference.manifest_digest.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let observed_digests = loaded
        .iter()
        .map(|reference| {
            (
                reference.id.to_ascii_lowercase(),
                reference.manifest_digest.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if expected_ids != observed_ids || expected_digests != observed_digests {
        bail!("skill_runtime_load_mismatch");
    }
    Ok(SkillLoadAudit {
        requested_skill_refs: resolved.requested_skill_refs.clone(),
        resolved_skill_refs: resolved.resolved_skill_refs.clone(),
        resolved_manifest_digest: resolved.resolved_manifest_digest.clone(),
        runtime_loaded_skill_refs: loaded,
        protocol: resolved.protocol.clone(),
        resolution_status: "loaded".to_string(),
        resolution_error_code: None,
    })
}

fn skill_protocol(capabilities: &[String]) -> &'static str {
    if capabilities.iter().any(|value| value == "agent-skill-v1") {
        "agent-skill-v1"
    } else {
        "skill-bundles-v1"
    }
}

fn effective_skill_refs(bindings: &SkillBindings) -> anyhow::Result<Vec<SkillReference>> {
    // Task selection is explicit and therefore replaces agent defaults. Stable
    // IDs deduplicate references while conflicting digests fail closed.
    let sources = if !bindings.task.is_empty() {
        vec![&bindings.task]
    } else {
        vec![&bindings.agent]
    };
    let mut by_id = BTreeMap::<String, SkillReference>::new();
    for source in sources {
        for reference in source {
            validate_reference(reference)?;
            if let Some(previous) = by_id.get(&reference.id) {
                if previous.manifest_digest != reference.manifest_digest {
                    bail!("skill_reference_conflict");
                }
                continue;
            }
            by_id.insert(reference.id.clone(), reference.clone());
        }
    }
    Ok(by_id.into_values().collect())
}

fn inventory_map(
    entries: &[SkillInventoryEntry],
) -> anyhow::Result<BTreeMap<String, SkillInventoryEntry>> {
    if entries.len() > MAX_SKILL_REFS {
        bail!("skill_inventory_too_large");
    }
    let mut map = BTreeMap::new();
    for entry in entries {
        validate_skill_id(&entry.id)?;
        if entry.name.trim().is_empty() || entry.name.len() > MAX_SKILL_ID_LENGTH {
            bail!("skill_inventory_invalid");
        }
        if let Some(digest) = entry.manifest_digest.as_deref() {
            validate_digest(digest)?;
        }
        if map.insert(entry.id.clone(), entry.clone()).is_some() {
            bail!("skill_inventory_conflict");
        }
    }
    Ok(map)
}

fn validate_capabilities(capabilities: &[String]) -> anyhow::Result<()> {
    if capabilities.len() > 64 {
        bail!("runtime_capabilities_invalid");
    }
    if !capabilities
        .iter()
        .any(|capability| capability == "skill-bundles-v1" || capability == "agent-skill-v1")
    {
        bail!("runtime_skills_unsupported");
    }
    if capabilities.iter().any(|capability| {
        capability.is_empty()
            || capability.len() > 80
            || capability
                .bytes()
                .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')))
    }) {
        bail!("runtime_capabilities_invalid");
    }
    Ok(())
}

fn validate_refs(refs: &[SkillReference]) -> anyhow::Result<()> {
    if refs.len() > MAX_SKILL_REFS {
        bail!("skill_refs_too_large");
    }
    let mut ids = BTreeSet::new();
    for reference in refs {
        validate_reference(reference)?;
        if !ids.insert(reference.id.to_ascii_lowercase()) {
            bail!("skill_reference_conflict");
        }
    }
    Ok(())
}

fn validate_reference(reference: &SkillReference) -> anyhow::Result<()> {
    validate_skill_id(&reference.id)?;
    if let Some(digest) = reference.manifest_digest.as_deref() {
        validate_digest(digest)?;
    }
    Ok(())
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

fn manifest_digest(
    refs: &[SkillReference],
    inventory: &BTreeMap<String, SkillInventoryEntry>,
) -> String {
    let mut hasher = Sha256::new();
    for reference in refs {
        hasher.update(reference.id.as_bytes());
        hasher.update([0]);
        if let Some(entry) = inventory.get(&reference.id) {
            hasher.update(entry.manifest_digest.as_deref().unwrap_or("").as_bytes());
            hasher.update([0]);
            hasher.update(entry.version.as_deref().unwrap_or("").as_bytes());
        }
        hasher.update([0xff]);
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(id: &str) -> SkillReference {
        SkillReference {
            id: id.to_string(),
            manifest_digest: Some("abc123".to_string()),
        }
    }

    fn inventory(id: &str) -> SkillInventoryEntry {
        SkillInventoryEntry {
            id: id.to_string(),
            name: id.to_string(),
            version: Some("1.0.0".to_string()),
            summary: Some("summary".to_string()),
            installed: true,
            trusted: true,
            compatible: true,
            manifest_digest: Some("abc123".to_string()),
        }
    }

    fn request(bindings: SkillBindings) -> SkillResolutionRequest {
        SkillResolutionRequest {
            bindings,
            runtime_capabilities: vec!["skill-bundles-v1".to_string()],
            inventory: vec![inventory("skill:a"), inventory("skill:b")],
        }
    }

    #[test]
    fn resolution_adds_inventory_digest_and_selects_native_protocol() {
        let resolved = resolve_skill_bindings(&request(SkillBindings {
            task: vec![SkillReference {
                id: "skill:a".to_string(),
                manifest_digest: None,
            }],
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(resolved.protocol, "skill-bundles-v1");
        assert_eq!(
            resolved.resolved_skill_refs[0].manifest_digest.as_deref(),
            Some("abc123")
        );
        let execution = resolved.execution_request().unwrap();
        assert_eq!(execution.protocol, "skill-bundles-v1");
        assert_eq!(execution.skill_refs, resolved.resolved_skill_refs);
        assert_eq!(execution.manifest_digest, resolved.resolved_manifest_digest);

        let mut request = request(SkillBindings {
            task: vec![SkillReference {
                id: "skill:a".to_string(),
                manifest_digest: None,
            }],
            ..Default::default()
        });
        request.runtime_capabilities = vec!["agent-skill-v1".to_string()];
        assert_eq!(
            resolve_skill_bindings(&request).unwrap().protocol,
            "agent-skill-v1"
        );
    }

    #[test]
    fn task_bindings_override_defaults_and_are_stably_deduplicated() {
        let resolved = resolve_skill_bindings(&request(SkillBindings {
            task: vec![reference("skill:b"), reference("skill:b")],
            agent: vec![reference("skill:a")],
            ..SkillBindings::default()
        }))
        .unwrap();
        assert_eq!(resolved.resolved_skill_refs, vec![reference("skill:b")]);
        assert_eq!(resolved.requested_skill_refs, resolved.resolved_skill_refs);
        assert!(resolved.resolved_manifest_digest.starts_with("sha256:"));
    }

    #[test]
    fn untrusted_unknown_incompatible_and_digest_conflicts_are_blocked() {
        let mut untrusted = inventory("skill:a");
        untrusted.trusted = false;
        let mut req = request(SkillBindings {
            task: vec![reference("skill:a")],
            ..Default::default()
        });
        req.inventory = vec![untrusted];
        assert_eq!(
            resolve_skill_bindings(&req).unwrap_err().to_string(),
            "skill_not_trusted"
        );

        req.inventory = vec![inventory("skill:a")];
        req.bindings.task = vec![reference("skill:missing")];
        assert_eq!(
            resolve_skill_bindings(&req).unwrap_err().to_string(),
            "skill_unknown"
        );

        req.bindings.task = vec![reference("skill:a")];
        req.inventory[0].compatible = false;
        assert_eq!(
            resolve_skill_bindings(&req).unwrap_err().to_string(),
            "skill_incompatible"
        );

        req.inventory[0].compatible = true;
        req.bindings.task[0].manifest_digest = Some("deadbeef".to_string());
        assert_eq!(
            resolve_skill_bindings(&req).unwrap_err().to_string(),
            "skill_manifest_conflict"
        );
    }

    #[test]
    fn runtime_loaded_set_is_a_separate_audit_and_mismatch_blocks() {
        let resolved = resolve_skill_bindings(&request(SkillBindings {
            task: vec![reference("skill:a")],
            ..Default::default()
        }))
        .unwrap();
        let loaded = record_runtime_loaded(&resolved, vec![reference("skill:a")]).unwrap();
        assert_eq!(loaded.resolution_status, "loaded");
        assert_eq!(loaded.requested_skill_refs, resolved.requested_skill_refs);
        assert_eq!(
            record_runtime_loaded(&resolved, vec![reference("skill:b")])
                .unwrap_err()
                .to_string(),
            "skill_runtime_load_mismatch"
        );
    }

    #[test]
    fn runtime_loaded_manifest_digest_mismatch_is_blocked() {
        let resolved = resolve_skill_bindings(&request(SkillBindings {
            task: vec![reference("skill:a")],
            ..Default::default()
        }))
        .unwrap();
        let mut changed = reference("skill:a");
        changed.manifest_digest = Some("deadbeef".to_string());
        assert_eq!(
            record_runtime_loaded(&resolved, vec![changed])
                .unwrap_err()
                .to_string(),
            "skill_runtime_load_mismatch"
        );

        let mut missing = reference("skill:a");
        missing.manifest_digest = None;
        assert_eq!(
            record_runtime_loaded(&resolved, vec![missing])
                .unwrap_err()
                .to_string(),
            "skill_runtime_load_mismatch"
        );
    }

    #[test]
    fn runtime_loaded_ids_are_case_insensitive_for_duplicate_detection() {
        let resolved = resolve_skill_bindings(&request(SkillBindings {
            task: vec![reference("skill:a")],
            ..Default::default()
        }))
        .unwrap();
        let loaded = vec![reference("skill:a"), reference("SKILL:A")];
        assert_eq!(
            record_runtime_loaded(&resolved, loaded)
                .unwrap_err()
                .to_string(),
            "skill_reference_conflict"
        );
    }

    #[test]
    fn no_skill_capability_blocks_before_inventory_use() {
        let mut req = request(SkillBindings {
            task: vec![reference("skill:a")],
            ..Default::default()
        });
        req.runtime_capabilities = vec!["rpc-v1".to_string()];
        assert_eq!(
            resolve_skill_bindings(&req).unwrap_err().to_string(),
            "runtime_skills_unsupported"
        );
    }

    #[test]
    fn skill_binding_scope_has_canonical_wire_names() {
        assert_eq!(SkillBindingScope::Task.as_str(), "task");
        assert_eq!(SkillBindingScope::Agent.as_str(), "agent");
        let encoded = serde_json::to_value(SkillBindings::default()).unwrap();
        let object = encoded.as_object().unwrap();
        assert_eq!(object.len(), 2);
        assert!(object.contains_key("task"));
        assert!(object.contains_key("agent"));
        let mut obsolete = serde_json::Map::new();
        obsolete.insert(["squad", "member"].concat(), serde_json::json!([]));
        assert!(serde_json::from_value::<SkillBindings>(obsolete.into()).is_err());
    }

    #[test]
    fn sha256_manifest_digest_from_local_inventory_is_accepted() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let mut req = request(SkillBindings {
            task: vec![SkillReference {
                id: "skill:a".to_string(),
                manifest_digest: Some(digest.clone()),
            }],
            ..Default::default()
        });
        req.inventory[0].manifest_digest = Some(digest.clone());
        let resolved = resolve_skill_bindings(&req).unwrap();
        assert_eq!(
            resolved.resolved_skill_refs[0].manifest_digest.as_deref(),
            Some(digest.as_str())
        );
    }
}
