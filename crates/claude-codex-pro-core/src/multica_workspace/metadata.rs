use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalWorkspaceIssueStatusReorder {
    pub category: String,
    #[serde(rename = "ids")]
    pub ordered_ids: Vec<String>,
    pub expected_revisions: BTreeMap<String, u64>,
}

impl LocalMulticaWorkspaceStore {
    pub fn require_agent_owner(&self, workspace_id: &str, agent_id: &str) -> anyhow::Result<()> {
        let state = self.load(workspace_id)?;
        let agent = state
            .agents
            .iter()
            .find(|agent| agent["id"].as_str() == Some(agent_id))
            .ok_or_else(|| anyhow!("multica_workspace_agent_not_found"))?;
        if owner(agent, "owner_id", &caller(workspace_id)) != caller(workspace_id) {
            bail!("multica_workspace_agent_access_owner_required");
        }
        Ok(())
    }

    /// Reorder the complete active custom category, preserving its system row.
    /// All revisions and category membership are checked before the single save.
    pub fn reorder_issue_statuses(
        &self,
        workspace_id: &str,
        command: &LocalWorkspaceIssueStatusReorder,
        updated_at_ms: u64,
        receipt: Option<&WorkspaceCommand>,
    ) -> anyhow::Result<Vec<Value>> {
        validate_local_workspace_id(workspace_id)?;
        if !is_issue_status_category(&command.category)
            || command.ordered_ids.len() > MAX_LOCAL_ENTITIES_PER_RESOURCE
        {
            bail!("multica_workspace_issue_status_reorder_invalid");
        }
        let _guard = local_workspace_store_lock(&self.path)?;
        let mut state = load_local_workspace_state(&self.path, workspace_id)?;
        if let Some(result) = replay_workspace_command(&state, receipt)? {
            return serde_json::from_value(result["statuses"].clone())
                .map_err(|_| anyhow!("multica_workspace_command_invalid"));
        }
        let active = state
            .issue_statuses
            .iter()
            .filter(|row| {
                row["category"].as_str() == Some(&command.category)
                    && !issue_status_is_archived(row)
                    && row["is_system"] == false
            })
            .map(|row| row["id"].as_str().unwrap_or_default())
            .collect::<BTreeSet<_>>();
        let requested = command
            .ordered_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if active != requested
            || requested.len() != command.ordered_ids.len()
            || command
                .expected_revisions
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != requested
        {
            bail!("multica_workspace_revision_conflict");
        }
        let mut statuses = Vec::with_capacity(requested.len());
        for (position, id) in command.ordered_ids.iter().enumerate() {
            let row = state
                .issue_statuses
                .iter_mut()
                .find(|row| row["id"].as_str() == Some(id))
                .unwrap();
            let revision = row["revision"].as_u64().unwrap_or_default();
            if command.expected_revisions.get(id) != Some(&revision) {
                bail!("multica_workspace_revision_conflict");
            }
            row["position"] = json!(position + 1);
            row["revision"] = json!(
                revision
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("multica_workspace_revision_conflict"))?
            );
            row["updated_at_ms"] = json!(updated_at_ms);
            statuses.push(row.clone());
        }
        record_workspace_command(
            &mut state,
            receipt,
            json!({"status":"ok","statuses":statuses}),
            false,
        )?;
        save_local_workspace_state_locked(&self.path, &state)?;
        Ok(statuses)
    }

    pub fn require_agent_invocation(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> anyhow::Result<()> {
        let state = self.load(workspace_id)?;
        let agent = state
            .agents
            .iter()
            .find(|agent| agent["id"].as_str() == Some(agent_id))
            .ok_or_else(|| anyhow!("multica_workspace_agent_not_found"))?;
        if !agent_invocable(agent, workspace_id) {
            bail!("multica_workspace_agent_access_denied");
        }
        Ok(())
    }
}

fn caller(workspace_id: &str) -> String {
    format!("{workspace_id}-user")
}

fn owner<'a>(row: &'a Value, field: &str, fallback: &'a str) -> &'a str {
    row.get(field)
        .or_else(|| {
            if field == "created_by_id" {
                row.get("createdById")
            } else {
                None
            }
        })
        .and_then(Value::as_str)
        .unwrap_or(fallback)
}

pub(super) fn agent_invocable(agent: &Value, workspace_id: &str) -> bool {
    let user = caller(workspace_id);
    if owner(agent, "owner_id", &user) == user {
        return true;
    }
    let explicit = agent["permission_mode"].as_str();
    if !matches!(explicit, Some("private" | "public_to")) {
        return agent["visibility"] == "workspace";
    }
    explicit == Some("public_to")
        && agent["invocation_targets"]
            .as_array()
            .is_some_and(|targets| {
                targets
                    .iter()
                    .any(|target| match target["target_type"].as_str() {
                        Some("workspace") => {
                            target["target_id"].is_null() || target["target_id"] == workspace_id
                        }
                        Some("member") => target["target_id"] == user,
                        _ => false,
                    })
            })
}

fn autopilot_writable(row: &Value, user: &str) -> bool {
    owner(row, "created_by_id", user) == user
        || row["collaborators"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry["user_id"] == user || entry["userId"] == user)
        })
}

pub(super) fn prepare_write(
    state: &LocalMulticaWorkspaceState,
    resource: MulticaWorkspaceResourceKey,
    existing: Option<&Value>,
    object: &mut serde_json::Map<String, Value>,
    expected_revision: Option<u64>,
    now: u64,
) -> anyhow::Result<()> {
    let user = caller(&state.workspace_id);
    if object
        .get("workspace_id")
        .is_some_and(|id| id != &state.workspace_id)
    {
        bail!("multica_workspace_tenant_mismatch");
    }
    match resource {
        MulticaWorkspaceResourceKey::Properties => {
            // The embedded local device user owns this catalog; no cloud admin role is inferred.
            if existing.is_none() && expected_revision != Some(0) {
                bail!("multica_workspace_revision_conflict");
            }
            if existing.is_some_and(|old| old.get("type") != object.get("type")) {
                bail!("multica_workspace_property_type_immutable");
            }
            object.entry("description").or_insert(json!(""));
            object.entry("icon").or_insert(json!(""));
            object.entry("config").or_insert(json!({}));
            object
                .entry("position")
                .or_insert(json!(state.properties.len()));
            object.entry("archived").or_insert(json!(false));
            let archived = object.get("archived").and_then(Value::as_bool) == Some(true);
            let archived_at = if archived {
                existing
                    .and_then(|old| old.get("archived_at"))
                    .filter(|value| !value.is_null())
                    .cloned()
                    .unwrap_or_else(|| {
                        json!(
                            chrono::DateTime::from_timestamp_millis(now as i64)
                                .unwrap_or_default()
                                .to_rfc3339()
                        )
                    })
            } else {
                Value::Null
            };
            object.insert("archived_at".into(), archived_at);
            object.remove("usage_count");
        }
        MulticaWorkspaceResourceKey::IssueViewPreferences => {
            if existing.is_none() && expected_revision != Some(0) {
                bail!("multica_workspace_revision_conflict");
            }
            if object.get("user_id").is_some_and(|id| id != &user)
                || existing.is_some_and(|row| row["user_id"] != user)
            {
                bail!("multica_workspace_preference_access_denied");
            }
            object.insert("user_id".into(), json!(user));
            object.entry("scope_id").or_insert(Value::Null);
            if existing.is_some_and(|row| {
                ["scope_type", "scope_id"]
                    .iter()
                    .any(|key| row.get(*key) != object.get(*key))
            }) {
                bail!("multica_workspace_preference_scope_immutable");
            }
            if object.get("scope_type").and_then(Value::as_str) == Some("project")
                && !state
                    .projects
                    .iter()
                    .any(|project| project.get("id") == object.get("scope_id"))
            {
                bail!("multica_workspace_preference_project_missing");
            }
        }
        MulticaWorkspaceResourceKey::Agents => {
            if existing.is_some_and(|row| owner(row, "owner_id", &user) != user) {
                bail!("multica_workspace_agent_access_owner_required");
            }
            if existing.is_some() && object.get("owner_id").is_some_and(|id| id != &user) {
                bail!("multica_workspace_agent_owner_invalid");
            }
            if let Some(targets) = object.get("invocation_targets").and_then(Value::as_array) {
                for target in targets {
                    if target["target_type"] == "team"
                        || (target["target_type"] == "member" && target["target_id"] != user)
                    {
                        bail!("multica_workspace_agent_invocation_target_invalid");
                    }
                }
            }
        }
        MulticaWorkspaceResourceKey::Autopilots | MulticaWorkspaceResourceKey::QuickActions => {
            let creator = existing
                .map(|row| owner(row, "created_by_id", &user))
                .unwrap_or(&user);
            if object.get("created_by_id").is_some_and(|id| id != creator)
                || object.get("createdById").is_some_and(|id| id != creator)
            {
                bail!("multica_workspace_creator_immutable");
            }
            if let Some(old) = existing {
                if resource == MulticaWorkspaceResourceKey::QuickActions && creator != user {
                    bail!("multica_workspace_quick_action_access_denied");
                }
                if resource == MulticaWorkspaceResourceKey::Autopilots {
                    if !autopilot_writable(old, &user) {
                        bail!("multica_workspace_autopilot_access_denied");
                    }
                    if creator != user && old.get("collaborators") != object.get("collaborators") {
                        bail!("multica_workspace_autopilot_creator_required");
                    }
                }
            }
            object.remove("createdById");
            object.insert("created_by_id".into(), json!(creator));
            object.remove("can_write");
            object.remove("can_manage_access");
            if resource == MulticaWorkspaceResourceKey::Autopilots {
                let changed = existing
                    .is_none_or(|row| row.get("collaborators") != object.get("collaborators"));
                if changed
                    && let Some(entries) = object.get("collaborators").and_then(Value::as_array)
                {
                    let mut seen = BTreeSet::new();
                    for entry in entries {
                        let id = entry
                            .get("user_id")
                            .or_else(|| entry.get("userId"))
                            .and_then(Value::as_str);
                        if id != Some(user.as_str())
                            || !seen.insert(id)
                            || entry.get("role").is_some_and(|role| role != "collaborator")
                        {
                            bail!("multica_workspace_autopilot_collaborator_invalid");
                        }
                    }
                }
            } else {
                object.entry("description").or_insert(json!(""));
                for (field, limit, required) in [("name", 32, true), ("description", 200, false)] {
                    let value = object
                        .get(field)
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow!("multica_workspace_quick_action_invalid"))?
                        .trim();
                    if value.chars().count() > limit || required && value.is_empty() {
                        bail!("multica_workspace_quick_action_invalid");
                    }
                    object.insert(field.into(), json!(value));
                }
                let prompt = native_domain::validated_quick_action_prompt(
                    object
                        .get("prompt")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow!("multica_workspace_quick_action_invalid"))?,
                )?;
                object.insert("prompt".into(), json!(prompt));
                object.entry("visibility").or_insert(json!("private"));
                object.entry("status").or_insert(json!("active"));
                for (field, default) in [("use_count", json!(0)), ("last_used_at", Value::Null)] {
                    object.insert(
                        field.into(),
                        existing
                            .and_then(|row| row.get(field))
                            .cloned()
                            .unwrap_or(default),
                    );
                }
                let target_id = object.get("assignee_id").and_then(Value::as_str);
                let is_agent = object.get("assignee_type").and_then(Value::as_str) == Some("agent");
                let targets = if is_agent {
                    &state.agents
                } else {
                    &state.squads
                };
                let target = targets
                    .iter()
                    .find(|target| target["id"].as_str() == target_id)
                    .ok_or_else(|| anyhow!("multica_workspace_quick_action_target_missing"))?;
                if is_agent && !agent_invocable(target, &state.workspace_id) {
                    bail!("multica_workspace_agent_access_denied");
                }
                if object.get("visibility").and_then(Value::as_str) == Some("public")
                    && !target_public(target)
                {
                    bail!("multica_workspace_quick_action_target_private");
                }
                for field in ["target_name", "target_public", "target_missing"] {
                    object.remove(field);
                }
            }
        }
        MulticaWorkspaceResourceKey::Subscribers | MulticaWorkspaceResourceKey::Reactions => {
            let agent_subscription = resource == MulticaWorkspaceResourceKey::Subscribers
                && object.get("user_type").and_then(Value::as_str) == Some("agent");
            let field = if resource == MulticaWorkspaceResourceKey::Subscribers {
                "user_id"
            } else {
                "actor_id"
            };
            let type_field = if resource == MulticaWorkspaceResourceKey::Subscribers {
                "user_type"
            } else {
                "actor_type"
            };
            if agent_subscription {
                let id = object
                    .get("user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !state
                    .agents
                    .iter()
                    .any(|agent| agent["id"] == id && owner(agent, "owner_id", &user) == user)
                {
                    bail!("multica_workspace_collaboration_access_denied");
                }
            } else {
                if object.get(field).is_some_and(|id| id != &user)
                    || object.get(type_field).is_some_and(|kind| kind != "member")
                {
                    bail!("multica_workspace_collaboration_access_denied");
                }
                object.insert(field.into(), json!(user));
                object.insert(type_field.into(), json!("member"));
            }
            if existing.is_some_and(|row| {
                row.get(field) != object.get(field) || row.get(type_field) != object.get(type_field)
            }) {
                bail!("multica_workspace_collaboration_identity_immutable");
            }
            let issue_id = object.get("issue_id").and_then(Value::as_str);
            if issue_id.is_some_and(|id| !state.issues.iter().any(|row| row["id"] == id)) {
                bail!("multica_workspace_collaboration_target_missing");
            }
            if let Some(comment_id) = object.get("comment_id").and_then(Value::as_str) {
                let comment = state
                    .comments
                    .iter()
                    .find(|row| row["id"] == comment_id)
                    .ok_or_else(|| anyhow!("multica_workspace_collaboration_target_missing"))?;
                if issue_id.is_some_and(|id| comment["issue_id"] != id) {
                    bail!("multica_workspace_collaboration_target_missing");
                }
            }
            let fields: &[&str] = if resource == MulticaWorkspaceResourceKey::Subscribers {
                &["issue_id", "user_id", "user_type"]
            } else {
                &["issue_id", "comment_id", "actor_id", "emoji"]
            };
            for row in state.collection(resource)? {
                if row.get("id") != object.get("id")
                    && fields.iter().all(|key| row.get(*key) == object.get(*key))
                {
                    bail!("multica_workspace_entity_conflict");
                }
            }
        }
        MulticaWorkspaceResourceKey::Issues => {
            if !object.contains_key("properties")
                && let Some(bag) = existing.and_then(|row| row.get("properties"))
            {
                object.insert("properties".into(), bag.clone());
            }
            if object.get("assignee_type").and_then(Value::as_str) == Some("agent") {
                if let Some(agent) = state
                    .agents
                    .iter()
                    .find(|agent| agent.get("id") == object.get("assignee_id"))
                {
                    if !agent_invocable(agent, &state.workspace_id) {
                        bail!("multica_workspace_agent_access_denied");
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_delete(
    state: &LocalMulticaWorkspaceState,
    resource: MulticaWorkspaceResourceKey,
    row: &Value,
) -> anyhow::Result<()> {
    let user = caller(&state.workspace_id);
    let permitted = match resource {
        MulticaWorkspaceResourceKey::Agents => owner(row, "owner_id", &user) == user,
        MulticaWorkspaceResourceKey::Autopilots | MulticaWorkspaceResourceKey::QuickActions => {
            owner(row, "created_by_id", &user) == user
        }
        MulticaWorkspaceResourceKey::Subscribers => {
            (row["user_type"] == "member" && row["user_id"] == user)
                || (row["user_type"] == "agent"
                    && state.agents.iter().any(|agent| {
                        agent["id"] == row["user_id"] && owner(agent, "owner_id", &user) == user
                    }))
        }
        MulticaWorkspaceResourceKey::IssueViewPreferences => row["user_id"] == user,
        MulticaWorkspaceResourceKey::Reactions => row["actor_id"] == user,
        _ => true,
    };
    if !permitted {
        bail!("multica_workspace_access_denied");
    }
    Ok(())
}

fn bounded_string(value: Option<&Value>, max: usize, nonempty: bool) -> bool {
    value.and_then(Value::as_str).is_some_and(|s| {
        s.chars().count() <= max && !s.contains('\0') && (!nonempty || !s.trim().is_empty())
    })
}

fn unique_strings(value: &Value, max: usize) -> bool {
    value.as_array().is_some_and(|items| {
        let mut seen = BTreeSet::new();
        items.len() <= max
            && items
                .iter()
                .all(|v| bounded_string(Some(v), 240, true) && seen.insert(v.as_str().unwrap()))
    })
}

pub(super) fn validate_contract(
    object: &serde_json::Map<String, Value>,
    resource: MulticaWorkspaceResourceKey,
) -> anyhow::Result<()> {
    match resource {
        MulticaWorkspaceResourceKey::Properties => {
            if !bounded_string(object.get("name"), 128, true)
                || !bounded_string(object.get("description"), 2048, false)
                || !bounded_string(object.get("icon"), 128, false)
                || object.get("position").and_then(Value::as_u64).is_none()
                || object.get("archived").and_then(Value::as_bool).is_none()
                || !matches!(
                    object.get("type").and_then(Value::as_str),
                    Some(
                        "text"
                            | "number"
                            | "select"
                            | "multi_select"
                            | "date"
                            | "checkbox"
                            | "url"
                            | "actor"
                            | "multi_actor"
                    )
                )
            {
                bail!("multica_workspace_property_invalid");
            }
            let config = object
                .get("config")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("multica_workspace_property_config_invalid"))?;
            if config.keys().any(|key| key != "options") {
                bail!("multica_workspace_property_config_invalid");
            }
            if let Some(options) = config.get("options") {
                let options = options
                    .as_array()
                    .ok_or_else(|| anyhow!("multica_workspace_property_config_invalid"))?;
                let mut ids = BTreeSet::new();
                if options.len() > 256 {
                    bail!("multica_workspace_property_config_invalid");
                }
                for option in options {
                    let id = option["id"].as_str().unwrap_or_default();
                    if validate_local_entity_id(id).is_err()
                        || !ids.insert(id)
                        || !bounded_string(option.get("name"), 128, true)
                        || !option["color"].as_str().is_some_and(is_hex_color)
                        || option.as_object().is_none_or(|row| {
                            row.keys()
                                .any(|key| !matches!(key.as_str(), "id" | "name" | "color"))
                        })
                    {
                        bail!("multica_workspace_property_config_invalid");
                    }
                }
            }
        }
        MulticaWorkspaceResourceKey::IssueViewPreferences => {
            let scope = object.get("scope_type").and_then(Value::as_str);
            let scope_id = object.get("scope_id");
            if !matches!(scope, Some("workspace" | "my" | "project"))
                || object.get("user_id").and_then(Value::as_str).is_none()
                || (scope == Some("project") && !bounded_string(scope_id, 240, true))
                || (scope != Some("project") && scope_id != Some(&Value::Null))
            {
                bail!("multica_workspace_preference_invalid");
            }
            let prefs = object
                .get("prefs")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("multica_workspace_preference_invalid"))?;
            if prefs.len() != 2
                || ["hidden", "order"]
                    .iter()
                    .any(|key| prefs.get(*key).is_none_or(|v| !unique_strings(v, 256)))
            {
                bail!("multica_workspace_preference_invalid");
            }
        }
        MulticaWorkspaceResourceKey::QuickActions => {
            if !bounded_string(object.get("name"), 128, true)
                || !bounded_string(object.get("description"), 2048, false)
                || !bounded_string(object.get("prompt"), 64000, true)
                || !bounded_string(object.get("assignee_id"), 240, true)
                || !matches!(
                    object.get("assignee_type").and_then(Value::as_str),
                    Some("agent" | "squad")
                )
                || !matches!(
                    object.get("visibility").and_then(Value::as_str),
                    Some("private" | "public")
                )
                || !matches!(
                    object.get("status").and_then(Value::as_str),
                    Some("active" | "archived")
                )
            {
                bail!("multica_workspace_quick_action_invalid");
            }
            let prompt = object["prompt"].as_str().unwrap();
            if native_domain::quick_action_template_token(prompt) {
                bail!("multica_workspace_quick_action_template_invalid");
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_unique_preferences(
    state: &LocalMulticaWorkspaceState,
) -> anyhow::Result<()> {
    let mut scopes = BTreeSet::new();
    for row in &state.issue_view_preferences {
        if !scopes.insert((
            row["user_id"].as_str(),
            row["scope_type"].as_str(),
            row["scope_id"].as_str(),
        )) {
            bail!("multica_workspace_preference_conflict");
        }
    }
    Ok(())
}

pub(super) fn validate_issue_properties(
    state: &LocalMulticaWorkspaceState,
    existing: Option<&Value>,
    issue: &Value,
) -> anyhow::Result<()> {
    let Some(bag) = issue.get("properties") else {
        return Ok(());
    };
    let bag = bag
        .as_object()
        .ok_or_else(|| anyhow!("multica_workspace_issue_properties_invalid"))?;
    if bag.len() > 256 {
        bail!("multica_workspace_issue_properties_invalid");
    }
    for (id, value) in bag {
        // Preserve historical values when editing unrelated fields or archiving definitions.
        if existing
            .and_then(|old| old.get("properties"))
            .and_then(|bag| bag.get(id))
            == Some(value)
        {
            continue;
        }
        let definition = state
            .properties
            .iter()
            .find(|row| row["id"] == *id)
            .ok_or_else(|| anyhow!("multica_workspace_property_not_found"))?;
        if definition["archived"] == true {
            bail!("multica_workspace_property_archived");
        }
        let option_valid = |v: &Value| {
            v.as_str().is_some_and(|id| {
                definition["config"]["options"]
                    .as_array()
                    .is_some_and(|options| options.iter().any(|option| option["id"] == id))
            })
        };
        let actor_valid = |v: &Value| {
            v.as_str() == Some(format!("member:{}", caller(&state.workspace_id)).as_str())
        };
        let valid = match definition["type"].as_str() {
            Some("text") => bounded_string(Some(value), 64000, false),
            Some("number") => value.as_f64().is_some_and(f64::is_finite),
            Some("checkbox") => value.is_boolean(),
            Some("date") => value.as_str().is_some_and(|date| {
                is_iso_date(date) && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
            }),
            Some("url") => value.as_str().is_some_and(|text| {
                text.len() <= 8192
                    && url::Url::parse(text).is_ok_and(|url| {
                        matches!(url.scheme(), "http" | "https")
                            && url.host_str().is_some()
                            && url.username().is_empty()
                            && url.password().is_none()
                    })
            }),
            Some("select") => option_valid(value),
            Some("multi_select") => {
                unique_strings(value, 256) && value.as_array().unwrap().iter().all(option_valid)
            }
            Some("actor") => actor_valid(value),
            Some("multi_actor") => {
                unique_strings(value, 20) && value.as_array().unwrap().iter().all(actor_valid)
            }
            _ => false,
        };
        if !valid {
            bail!("multica_workspace_property_value_invalid");
        }
    }
    Ok(())
}

fn target_public(target: &Value) -> bool {
    target["visibility"] == "public"
        || (target["permission_mode"] == "public_to"
            && target["invocation_targets"]
                .as_array()
                .is_some_and(|targets| targets.iter().any(|t| t["target_type"] == "workspace")))
        || (target["permission_mode"].is_null() && target["visibility"] == "workspace")
}

pub(super) fn project_collection(
    state: &LocalMulticaWorkspaceState,
    resource: MulticaWorkspaceResourceKey,
    items: &mut Vec<Value>,
) {
    let user = caller(&state.workspace_id);
    match resource {
        MulticaWorkspaceResourceKey::IssueViewPreferences => {
            items.retain(|row| row["user_id"] == user)
        }
        MulticaWorkspaceResourceKey::Agents => {
            items.retain(|row| agent_invocable(row, &state.workspace_id))
        }
        MulticaWorkspaceResourceKey::Properties => {
            for row in items.iter_mut() {
                let id = row["id"].as_str().unwrap_or_default();
                let count = state
                    .issues
                    .iter()
                    .filter(|issue| issue["properties"].get(id).is_some())
                    .count();
                row["usage_count"] = json!(count);
            }
            items.sort_by_key(|row| row["position"].as_u64().unwrap_or_default());
        }
        MulticaWorkspaceResourceKey::QuickActions => {
            items.retain(|row| row["visibility"] == "public" || row["created_by_id"] == user);
            for row in items {
                let targets = if row["assignee_type"] == "squad" {
                    &state.squads
                } else {
                    &state.agents
                };
                let target = targets
                    .iter()
                    .find(|target| target["id"] == row["assignee_id"]);
                let missing = target.is_none_or(|target| {
                    target["status"] == "archived"
                        || target["archived"] == true
                        || target.get("archived_at").is_some_and(|v| !v.is_null())
                });
                row["target_missing"] = json!(missing);
                row["target_public"] = json!(!missing && target.is_some_and(target_public));
                if let Some(name) = target.and_then(|target| target.get("name")) {
                    row["target_name"] = name.clone();
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
