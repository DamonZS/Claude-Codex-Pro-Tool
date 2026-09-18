use super::*;
use tempfile::tempdir;

fn write(
    store: &LocalMulticaWorkspaceStore,
    resource: MulticaWorkspaceResourceKey,
    entity: Value,
    revision: u64,
) -> anyhow::Result<Value> {
    store.upsert(
        "local-test",
        LocalWorkspaceEntityUpsert {
            resource,
            entity,
            expected_revision: Some(revision),
        },
        123456,
    )
}

fn property(store: &LocalMulticaWorkspaceStore, id: &str, kind: &str) -> Value {
    write(
        store,
        MulticaWorkspaceResourceKey::Properties,
        json!({"id":id,"name":id,"type":kind}),
        0,
    )
    .unwrap()
}

fn preference(id: &str) -> Value {
    json!({"id":id,"scope_type":"my","prefs":{"hidden":["description"],"order":["title","status"]}})
}

#[test]
fn metadata_defaults_are_backward_compatible_and_persist() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let mut old = serde_json::to_value(LocalMulticaWorkspaceState::empty("local-test")).unwrap();
    for field in ["properties", "issueViewPreferences", "quickActions"] {
        old.as_object_mut().unwrap().remove(field);
    }
    fs::write(store.path(), serde_json::to_vec(&old).unwrap()).unwrap();
    let state = store.load("local-test").unwrap();
    assert!(
        state.properties.is_empty()
            && state.issue_view_preferences.is_empty()
            && state.quick_actions.is_empty()
    );
    let row = property(&store, "cost", "number");
    assert_eq!(row["config"], json!({}));
    assert_eq!(row["archived"], false);
    assert!(row["archived_at"].is_null());
    assert_eq!(store.load("local-test").unwrap().properties, vec![row]);
}

#[test]
fn metadata_preferences_enforce_identity_unique_scope_and_cas() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let row = write(
        &store,
        MulticaWorkspaceResourceKey::IssueViewPreferences,
        preference("pref-a"),
        0,
    )
    .unwrap();
    assert_eq!(row["user_id"], "local-test-user");
    assert!(row["scope_id"].is_null());
    assert_eq!(
        write(
            &store,
            MulticaWorkspaceResourceKey::IssueViewPreferences,
            preference("pref-b"),
            0
        )
        .unwrap_err()
        .to_string(),
        "multica_workspace_preference_conflict"
    );
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::IssueViewPreferences,
            row.clone(),
            0
        )
        .is_err()
    );
    let mut changed = row.clone();
    changed["prefs"]["hidden"] = json!([]);
    let changed = write(
        &store,
        MulticaWorkspaceResourceKey::IssueViewPreferences,
        changed,
        1,
    )
    .unwrap();
    assert_eq!(changed["revision"], 2);
    let mut spoof = preference("pref-other");
    spoof["user_id"] = json!("other-user");
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::IssueViewPreferences,
            spoof,
            0
        )
        .is_err()
    );
    let mut state = store.load("local-test").unwrap();
    let mut other = row;
    other["id"] = json!("other");
    other["user_id"] = json!("other-user");
    state.issue_view_preferences.push(other);
    store.save(&state).unwrap();
    assert_eq!(
        store
            .list(
                "local-test",
                MulticaWorkspaceResourceKey::IssueViewPreferences
            )
            .unwrap(),
        vec![changed]
    );
}

#[test]
fn metadata_preference_validation_rejects_bad_shapes_and_unknown_project() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    for prefs in [
        json!({"hidden":[],"order":["title","title"]}),
        json!({"hidden":"title","order":[]}),
        json!({"hidden":[],"order":[],"extra":true}),
    ] {
        let mut row = preference("prefs");
        row["prefs"] = prefs;
        assert!(
            write(
                &store,
                MulticaWorkspaceResourceKey::IssueViewPreferences,
                row,
                0
            )
            .is_err()
        );
    }
    let mut row = preference("prefs");
    row["scope_type"] = json!("project");
    row["scope_id"] = json!("project-a");
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::IssueViewPreferences,
            row.clone(),
            0
        )
        .is_err()
    );
    write(
        &store,
        MulticaWorkspaceResourceKey::Projects,
        json!({"id":"project-a"}),
        0,
    )
    .unwrap();
    write(
        &store,
        MulticaWorkspaceResourceKey::IssueViewPreferences,
        row,
        0,
    )
    .unwrap();
}

#[test]
fn metadata_property_types_reject_coercion_and_unknown_members() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    for (kind, valid, invalid) in [
        ("text", json!("notes"), json!(7)),
        ("number", json!(2.5), json!("2.5")),
        ("checkbox", json!(true), json!(1)),
        ("date", json!("2024-02-29"), json!("2025-02-29")),
        (
            "url",
            json!("https://example.com/a"),
            json!("javascript:alert(1)"),
        ),
        (
            "actor",
            json!("member:local-test-user"),
            json!("member:stranger"),
        ),
        (
            "multi_actor",
            json!(["member:local-test-user"]),
            json!("member:local-test-user"),
        ),
    ] {
        property(&store, kind, kind);
        let mut issue = json!({"id":format!("issue-{kind}"),"properties":{kind:valid}});
        let saved = write(
            &store,
            MulticaWorkspaceResourceKey::Issues,
            issue.clone(),
            0,
        )
        .unwrap();
        issue["properties"][kind] = invalid;
        assert_eq!(
            write(&store, MulticaWorkspaceResourceKey::Issues, issue, 1)
                .unwrap_err()
                .to_string(),
            "multica_workspace_property_value_invalid",
            "{kind}"
        );
        assert!(store.load("local-test").unwrap().issues.contains(&saved));
    }
    let unknown = json!({"id":"unknown","properties":{"not-defined":"value"}});
    assert!(write(&store, MulticaWorkspaceResourceKey::Issues, unknown, 0).is_err());
    let many = vec![json!("member:local-test-user"); 21];
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Issues,
            json!({"id":"many","properties":{"multi_actor":many}}),
            0
        )
        .is_err()
    );
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Issues,
            json!({"id":"bad-bag","properties":[]}),
            0
        )
        .is_err()
    );
}

#[test]
fn metadata_select_options_are_bounded_unique_and_type_immutable() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    for kind in ["select", "multi_select"] {
        let definition = json!({"id":kind,"name":kind,"type":kind,"config":{"options":[{"id":"red","name":"Red","color":"#ff0000"}]}});
        let mut duplicate = definition.clone();
        duplicate["config"]["options"]
            .as_array_mut()
            .unwrap()
            .push(definition["config"]["options"][0].clone());
        assert!(
            write(
                &store,
                MulticaWorkspaceResourceKey::Properties,
                duplicate,
                0
            )
            .is_err()
        );
        let mut saved = write(
            &store,
            MulticaWorkspaceResourceKey::Properties,
            definition,
            0,
        )
        .unwrap();
        saved["type"] = json!("text");
        assert_eq!(
            write(&store, MulticaWorkspaceResourceKey::Properties, saved, 1)
                .unwrap_err()
                .to_string(),
            "multica_workspace_property_type_immutable"
        );
        let valid = if kind == "select" {
            json!("red")
        } else {
            json!(["red"])
        };
        write(
            &store,
            MulticaWorkspaceResourceKey::Issues,
            json!({"id":kind,"properties":{kind:valid}}),
            0,
        )
        .unwrap();
        for invalid in [
            json!("missing"),
            json!(["missing"]),
            json!(["red", "red"]),
            Value::Null,
        ] {
            assert!(
                write(
                    &store,
                    MulticaWorkspaceResourceKey::Issues,
                    json!({"id":kind,"properties":{kind:invalid}}),
                    1
                )
                .is_err()
            );
        }
    }
}

#[test]
fn metadata_property_archive_preserves_values_usage_and_unrelated_edits() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let mut definition = property(&store, "cost", "number");
    property(&store, "note", "text");
    let mut issue = write(
        &store,
        MulticaWorkspaceResourceKey::Issues,
        json!({"id":"issue","properties":{"cost":1,"note":"retain"}}),
        0,
    )
    .unwrap();
    definition["archived"] = json!(true);
    let definition = write(
        &store,
        MulticaWorkspaceResourceKey::Properties,
        definition,
        1,
    )
    .unwrap();
    assert!(definition["archived_at"].is_string());
    issue["title"] = json!("new title");
    issue = write(&store, MulticaWorkspaceResourceKey::Issues, issue, 1).unwrap();
    assert_eq!(
        store
            .list("local-test", MulticaWorkspaceResourceKey::Properties)
            .unwrap()[0]["usage_count"],
        1
    );
    let mut changed = issue.clone();
    changed["properties"]["cost"] = json!(2);
    assert_eq!(
        write(&store, MulticaWorkspaceResourceKey::Issues, changed, 2)
            .unwrap_err()
            .to_string(),
        "multica_workspace_property_archived"
    );
    issue["properties"].as_object_mut().unwrap().remove("cost");
    let result = write(&store, MulticaWorkspaceResourceKey::Issues, issue, 2).unwrap();
    assert_eq!(result["properties"], json!({"note":"retain"}));
    assert_eq!(
        store
            .list("local-test", MulticaWorkspaceResourceKey::Properties)
            .unwrap()[0]["usage_count"],
        0
    );
    let updated = write(
        &store,
        MulticaWorkspaceResourceKey::Issues,
        json!({"id":"issue","title":"no bag"}),
        3,
    )
    .unwrap();
    assert_eq!(updated["properties"], json!({"note":"retain"}));
}

#[test]
fn metadata_status_reorder_is_atomic_category_complete_and_replayable() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    write(&store, MulticaWorkspaceResourceKey::IssueStatuses, json!({"id":"custom","name":"Ready","key":"ready","category":"todo","color":"#123456","position":1,"is_system":false}), 0).unwrap();
    write(&store, MulticaWorkspaceResourceKey::IssueStatuses, json!({"id":"second","name":"Next","key":"next","category":"todo","color":"#123456","position":2,"is_system":false}), 0).unwrap();
    let before = fs::read(store.path()).unwrap();
    let rows = store.load("local-test").unwrap().issue_statuses;
    let system = rows
        .iter()
        .find(|row| row["key"] == "todo")
        .unwrap()
        .clone();
    let mut command = LocalWorkspaceIssueStatusReorder {
        category: "todo".into(),
        ordered_ids: vec!["second".into(), "custom".into()],
        expected_revisions: BTreeMap::from([("second".into(), 1), ("custom".into(), 99)]),
    };
    assert!(
        store
            .reorder_issue_statuses("local-test", &command, 2, None)
            .is_err()
    );
    assert_eq!(fs::read(store.path()).unwrap(), before);
    command.expected_revisions.insert("custom".into(), 1);
    let receipt = WorkspaceCommand::new(
        "reorder-1".into(),
        "a".repeat(64),
        "reorder".into(),
        serde_json::to_value(&command).unwrap(),
    )
    .unwrap();
    let result = store
        .reorder_issue_statuses("local-test", &command, 2, Some(&receipt))
        .unwrap();
    assert_eq!(result[0]["id"], "second");
    assert_eq!(result[1]["position"], 2);
    assert!(
        store
            .load("local-test")
            .unwrap()
            .issue_statuses
            .contains(&system)
    );
    assert_eq!(
        store
            .reorder_issue_statuses("local-test", &command, 2, Some(&receipt))
            .unwrap(),
        result
    );
    assert!(
        store
            .reorder_issue_statuses("local-test", &command, 3, None)
            .is_err()
    );
    let current = fs::read(store.path()).unwrap();
    command.ordered_ids.pop();
    command.expected_revisions.remove("custom");
    assert!(
        store
            .reorder_issue_statuses("local-test", &command, 3, None)
            .is_err()
    );
    assert_eq!(fs::read(store.path()).unwrap(), current);
}

#[test]
fn metadata_agent_access_cannot_be_forged_and_foreign_private_agents_are_hidden() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let mut agent = write(
        &store,
        MulticaWorkspaceResourceKey::Agents,
        json!({"id":"agent","name":"A","permission_mode":"private"}),
        0,
    )
    .unwrap();
    agent["invocation_targets"] = json!([{"target_type":"member","target_id":"stranger"}]);
    agent["permission_mode"] = json!("public_to");
    assert!(write(&store, MulticaWorkspaceResourceKey::Agents, agent, 1).is_err());
    let mut state = store.load("local-test").unwrap();
    state.agents[0]["owner_id"] = json!("other");
    store.save(&state).unwrap();
    assert!(
        store
            .list("local-test", MulticaWorkspaceResourceKey::Agents)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .require_agent_invocation("local-test", "agent")
            .is_err()
    );
    let mut stolen = state.agents[0].clone();
    stolen["owner_id"] = json!("local-test-user");
    assert!(write(&store, MulticaWorkspaceResourceKey::Agents, stolen, 1).is_err());
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Issues,
            json!({"id":"issue","assignee_type":"agent","assignee_id":"agent"}),
            0
        )
        .is_err()
    );
    state.agents[0]["permission_mode"] = json!("public_to");
    state.agents[0]["invocation_targets"] = json!([{"target_type":"workspace"}]);
    store.save(&state).unwrap();
    store
        .require_agent_invocation("local-test", "agent")
        .unwrap();
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Agents,
            state.agents[0].clone(),
            1
        )
        .is_err()
    );
}

#[test]
fn metadata_autopilot_access_changes_require_creator_not_collaborator() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let row = write(
        &store,
        MulticaWorkspaceResourceKey::Autopilots,
        json!({"id":"pilot","name":"Pilot"}),
        0,
    )
    .unwrap();
    assert_eq!(row["created_by_id"], "local-test-user");
    let mut state = store.load("local-test").unwrap();
    state.autopilots[0]["created_by_id"] = json!("other");
    state.autopilots[0]["collaborators"] =
        json!([{"user_id":"local-test-user","role":"collaborator"}]);
    store.save(&state).unwrap();
    let mut row = state.autopilots[0].clone();
    row["name"] = json!("updated");
    let row = write(&store, MulticaWorkspaceResourceKey::Autopilots, row, 1).unwrap();
    let mut escalation = row.clone();
    escalation["collaborators"] = json!([]);
    assert_eq!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Autopilots,
            escalation,
            2
        )
        .unwrap_err()
        .to_string(),
        "multica_workspace_autopilot_creator_required"
    );
    let mut escalation = row;
    escalation["created_by_id"] = json!("local-test-user");
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Autopilots,
            escalation,
            2
        )
        .is_err()
    );
    assert!(
        store
            .delete(
                "local-test",
                LocalWorkspaceEntityDelete {
                    resource: MulticaWorkspaceResourceKey::Autopilots,
                    entity_id: "pilot".into(),
                    expected_revision: 2
                }
            )
            .is_err()
    );
}

#[test]
fn metadata_subscriptions_and_reactions_require_local_actor_existing_targets_and_cas() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    write(
        &store,
        MulticaWorkspaceResourceKey::Issues,
        json!({"id":"issue"}),
        0,
    )
    .unwrap();
    write(
        &store,
        MulticaWorkspaceResourceKey::Agents,
        json!({"id":"agent","name":"Agent"}),
        0,
    )
    .unwrap();
    write(
        &store,
        MulticaWorkspaceResourceKey::Subscribers,
        json!({"id":"agent-sub","issue_id":"issue","user_type":"agent","user_id":"agent"}),
        0,
    )
    .unwrap();
    assert!(
        store
            .delete(
                "local-test",
                LocalWorkspaceEntityDelete {
                    resource: MulticaWorkspaceResourceKey::Subscribers,
                    entity_id: "agent-sub".into(),
                    expected_revision: 1
                }
            )
            .unwrap()
    );
    write(
        &store,
        MulticaWorkspaceResourceKey::Comments,
        json!({"id":"comment","issue_id":"issue","content":"note"}),
        0,
    )
    .unwrap();
    for (resource, mut row, field) in [
        (
            MulticaWorkspaceResourceKey::Subscribers,
            json!({"id":"sub","issue_id":"issue"}),
            "user_id",
        ),
        (
            MulticaWorkspaceResourceKey::Reactions,
            json!({"id":"react","comment_id":"comment","emoji":"+1"}),
            "actor_id",
        ),
    ] {
        row[field] = json!("stranger");
        assert!(write(&store, resource, row.clone(), 0).is_err());
        row[field] = json!("local-test-user");
        let saved = write(&store, resource, row.clone(), 0).unwrap();
        let id = saved["id"].as_str().unwrap();
        row["id"] = json!(format!("duplicate-{id}"));
        assert!(write(&store, resource, row, 0).is_err());
        assert!(
            store
                .delete(
                    "local-test",
                    LocalWorkspaceEntityDelete {
                        resource,
                        entity_id: id.into(),
                        expected_revision: 99
                    }
                )
                .is_err()
        );
        assert!(
            store
                .delete(
                    "local-test",
                    LocalWorkspaceEntityDelete {
                        resource,
                        entity_id: id.into(),
                        expected_revision: 1
                    }
                )
                .unwrap()
        );
    }
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::Reactions,
            json!({"id":"bad","comment_id":"missing","emoji":"+1"}),
            0
        )
        .is_err()
    );
}

#[test]
fn metadata_quick_action_catalog_validates_target_and_derives_display_without_metrics() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let mut agent = write(
        &store,
        MulticaWorkspaceResourceKey::Agents,
        json!({"id":"agent","name":"Review","permission_mode":"private"}),
        0,
    )
    .unwrap();
    let action = json!({"id":"action","name":"Review","assignee_type":"agent","assignee_id":"agent","prompt":"Review the issue","use_count":999});
    let saved = write(&store, MulticaWorkspaceResourceKey::QuickActions, action, 0).unwrap();
    assert_eq!(saved["use_count"], 0);
    let mut public = saved.clone();
    public["visibility"] = json!("public");
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::QuickActions,
            public.clone(),
            1
        )
        .is_err()
    );
    agent["permission_mode"] = json!("public_to");
    agent["invocation_targets"] = json!([{"target_type":"workspace"}]);
    write(&store, MulticaWorkspaceResourceKey::Agents, agent, 1).unwrap();
    let mut updated = write(&store, MulticaWorkspaceResourceKey::QuickActions, public, 1).unwrap();
    updated["prompt"] = json!("Review {{title}}");
    assert!(
        write(
            &store,
            MulticaWorkspaceResourceKey::QuickActions,
            updated,
            2
        )
        .is_err()
    );
    let rows = store
        .list("local-test", MulticaWorkspaceResourceKey::QuickActions)
        .unwrap();
    assert_eq!(rows[0]["target_name"], "Review");
    assert_eq!(rows[0]["target_public"], true);
    assert_eq!(rows[0]["target_missing"], false);
    assert_eq!(rows[0]["use_count"], 0);
    assert!(rows[0]["last_used_at"].is_null());
}

#[test]
fn metadata_reorder_wire_contract_accepts_ids_and_rejects_extra_fields() {
    let payload = json!({"category":"todo","ids":["ready"],"expectedRevisions":{"ready":2}});
    let command: LocalWorkspaceIssueStatusReorder =
        serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(command.ordered_ids, vec!["ready"]);
    assert_eq!(serde_json::to_value(command).unwrap(), payload);
    let mut invalid = payload;
    invalid["owner"] = json!("admin");
    assert!(serde_json::from_value::<LocalWorkspaceIssueStatusReorder>(invalid).is_err());
}

#[test]
fn metadata_concurrent_first_preferences_have_one_durable_winner() {
    let dir = tempdir().unwrap();
    let store = LocalMulticaWorkspaceStore::new(dir.path().join("workspace.json"));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2)
        .map(|index| {
            let store = store.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                write(
                    &store,
                    MulticaWorkspaceResourceKey::IssueViewPreferences,
                    preference(&format!("pref-{index}")),
                    0,
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        store
            .load("local-test")
            .unwrap()
            .issue_view_preferences
            .len(),
        1
    );
}

#[test]
fn metadata_property_receipt_replay_survives_reopen_and_rejects_changed_payload() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("workspace.json");
    let store = LocalMulticaWorkspaceStore::new(path.clone());
    let entity = json!({"id":"text","type":"text","name":"Text"});
    let receipt = WorkspaceCommand::new(
        "property-create".into(),
        "b".repeat(64),
        "upsert:properties".into(),
        json!({"entity":entity}),
    )
    .unwrap();
    let command = LocalWorkspaceEntityUpsert {
        resource: MulticaWorkspaceResourceKey::Properties,
        entity,
        expected_revision: Some(0),
    };
    let saved = store
        .upsert_with_command("local-test", command.clone(), 10, Some(&receipt))
        .unwrap();
    let reopened = LocalMulticaWorkspaceStore::new(path);
    assert_eq!(
        reopened
            .upsert_with_command("local-test", command.clone(), 20, Some(&receipt))
            .unwrap(),
        saved
    );
    let altered = WorkspaceCommand::new(
        "property-create".into(),
        "c".repeat(64),
        "upsert:properties".into(),
        json!({"entity":command.entity}),
    )
    .unwrap();
    assert!(
        reopened
            .upsert_with_command("local-test", command, 20, Some(&altered))
            .is_err()
    );
    assert_eq!(reopened.load("local-test").unwrap().properties.len(), 1);
}
