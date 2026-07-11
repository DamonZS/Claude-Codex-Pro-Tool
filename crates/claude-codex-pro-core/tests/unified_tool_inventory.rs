use std::fs;

use claude_codex_pro_core::unified_tool_inventory::{
    UnifiedToolInventoryRoots, UnifiedToolToggleRequest, scan_unified_tool_inventory,
    set_unified_tool_asset_enabled,
};
use tempfile::tempdir;

fn write(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn unified_tool_inventory_toggles_only_the_requested_app_and_restores_skills() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_config = temp.path().join("claude.json");
    write(
        &codex_home.join("config.toml"),
        "[mcp_servers.shared]\ncommand = \"server\"\nenabled = true\n",
    );
    write(
        &claude_config,
        r#"{"mcpServers":{"shared":{"command":"server","enabled":true}}}"#,
    );
    write(
        &codex_home.join("skills/shared-skill/SKILL.md"),
        "---\nname: shared-skill\n---\n",
    );
    write(
        &claude_home.join("skills/shared-skill/SKILL.md"),
        "---\nname: shared-skill\n---\n",
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home: codex_home.clone(),
        claude_home: claude_home.clone(),
        claude_config_paths: vec![claude_config.clone()],
    };

    let after_mcp = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let mcp = after_mcp
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "shared")
        .unwrap();
    assert!(mcp.codex.enabled);
    assert!(!mcp.claude.enabled);

    set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-skill".to_string(),
            kind: "skill".to_string(),
            app: "codex".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    assert!(!codex_home.join("skills/shared-skill").exists());
    assert!(
        codex_home
            .join("skills/.ccp-disabled/shared-skill/SKILL.md")
            .exists()
    );
    assert!(claude_home.join("skills/shared-skill/SKILL.md").exists());

    set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-skill".to_string(),
            kind: "skill".to_string(),
            app: "codex".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    assert!(codex_home.join("skills/shared-skill/SKILL.md").exists());
}

#[test]
fn nested_skill_toggle_preserves_its_relative_directory() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let original = codex_home.join("skills/workflows/nested-folder/SKILL.md");
    write(
        &original,
        "---\nname: nested-skill\ndescription: Nested skill\n---\n",
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home: codex_home.clone(),
        claude_home,
        claude_config_paths: Vec::new(),
    };

    set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "nested-skill".to_string(),
            kind: "skill".to_string(),
            app: "codex".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let disabled = codex_home.join("skills/.ccp-disabled/workflows/nested-folder/SKILL.md");
    assert!(!original.exists());
    assert!(disabled.exists());

    set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "nested-skill".to_string(),
            kind: "skill".to_string(),
            app: "codex".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    assert!(original.exists());
    assert!(!disabled.exists());
    assert!(!codex_home.join("skills/nested-skill/SKILL.md").exists());
}

#[test]
fn unified_tool_inventory_aggregates_claude_configs_and_keeps_cached_plugins_disabled() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_primary = temp.path().join("claude-primary.json");
    let claude_msix = temp.path().join("claude-msix.json");
    write(
        &claude_primary,
        r#"{"mcpServers":{"primary":{"command":"primary-server"}}}"#,
    );
    write(
        &claude_msix,
        r#"{
  "mcpServers": {"msix":{"command":"msix-server"}},
  "projects": {
    "D:/workspace": {
      "mcpServers": {"project-only":{"command":"project-server"}}
    }
  }
}"#,
    );
    write(
        &claude_home.join(
            "plugins/cache/claude-plugins-official/cached-only/1.0.0/.claude-plugin/plugin.json",
        ),
        r#"{"name":"cached-only","description":"not enabled"}"#,
    );

    let inventory = scan_unified_tool_inventory(&UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: vec![claude_primary, claude_msix],
    })
    .unwrap();

    assert!(
        inventory
            .assets
            .iter()
            .any(|asset| asset.kind == "mcp" && asset.id == "primary")
    );
    assert!(
        inventory
            .assets
            .iter()
            .any(|asset| asset.kind == "mcp" && asset.id == "msix")
    );
    assert!(
        inventory
            .assets
            .iter()
            .any(|asset| asset.kind == "mcp" && asset.id == "project-only")
    );
    let cached = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "cached-only")
        .unwrap();
    assert!(cached.claude.available);
    assert!(!cached.claude.enabled);
}

#[test]
fn disabling_claude_mcp_removes_all_top_level_and_project_copies() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_primary = temp.path().join("claude-primary.json");
    let claude_msix = temp.path().join("claude-msix.json");
    write(
        &claude_primary,
        r#"{
  "mcpServers": {
    "shared": { "command": "primary-server" },
    "keep-top": { "command": "keep" }
  },
  "projects": {
    "D:/workspace-a": {
      "mcpServers": {
        "shared": { "command": "project-a-server" },
        "keep-project": { "command": "keep" }
      }
    }
  }
}"#,
    );
    write(
        &claude_msix,
        r#"{
  "projects": {
    "D:/workspace-b": {
      "mcpServers": {
        "shared": { "command": "project-b-server" }
      }
    }
  }
}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: vec![claude_primary.clone(), claude_msix.clone()],
    };

    let before = scan_unified_tool_inventory(&roots).unwrap();
    assert!(
        before
            .assets
            .iter()
            .any(|asset| asset.kind == "mcp" && asset.id == "shared" && asset.claude.enabled)
    );

    let after = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: false,
        },
    )
    .unwrap();

    let disabled_asset = after
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "shared")
        .expect("disabled Claude-only MCP must remain visible for restoration");
    assert!(!disabled_asset.claude.enabled);
    assert!(disabled_asset.claude.available);
    assert!(disabled_asset.claude.toggle_supported);
    let serialized = serde_json::to_string(&after).unwrap();
    assert!(!serialized.contains("restoreBody"));
    assert!(!serialized.contains("primary-server"));
    assert!(!serialized.contains("project-a-server"));
    assert!(!serialized.contains("project-b-server"));
    let primary: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&claude_primary).unwrap()).unwrap();
    let msix: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&claude_msix).unwrap()).unwrap();
    assert!(primary["mcpServers"].get("shared").is_none());
    assert!(
        primary["projects"]["D:/workspace-a"]["mcpServers"]
            .get("shared")
            .is_none()
    );
    assert!(
        msix["projects"]["D:/workspace-b"]["mcpServers"]
            .get("shared")
            .is_none()
    );
    assert_eq!(primary["mcpServers"]["keep-top"]["command"], "keep");
    assert_eq!(
        primary["projects"]["D:/workspace-a"]["mcpServers"]["keep-project"]["command"],
        "keep"
    );

    let restored = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let restored_asset = restored
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "shared")
        .unwrap();
    assert!(restored_asset.claude.enabled);
    let primary: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&claude_primary).unwrap()).unwrap();
    let msix: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&claude_msix).unwrap()).unwrap();
    assert_eq!(primary["mcpServers"]["shared"]["command"], "primary-server");
    assert_eq!(
        primary["projects"]["D:/workspace-a"]["mcpServers"]["shared"]["command"],
        "project-a-server"
    );
    assert_eq!(
        msix["projects"]["D:/workspace-b"]["mcpServers"]["shared"]["command"],
        "project-b-server"
    );
    assert!(msix["mcpServers"].get("shared").is_none());
}

#[test]
fn restoring_claude_mcp_refuses_to_overwrite_a_new_same_name_configuration() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_config = temp.path().join("claude.json");
    write(
        &claude_config,
        r#"{
  "mcpServers": {"shared":{"command":"old-server"}},
  "projects": {
    "D:/workspace": {
      "mcpServers": {"shared":{"command":"old-project-server"}}
    }
  }
}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: vec![claude_config.clone()],
    };

    set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    write(
        &claude_config,
        r#"{"mcpServers":{"shared":{"command":"new-server"}}}"#,
    );

    let error = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: true,
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("同名 MCP 配置冲突"));
    let current: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&claude_config).unwrap()).unwrap();
    assert_eq!(current["mcpServers"]["shared"]["command"], "new-server");
    assert!(current["projects"].is_null());
    let after = scan_unified_tool_inventory(&roots).unwrap();
    let asset = after
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "shared")
        .unwrap();
    assert!(asset.claude.enabled);
    assert!(
        !asset.claude.restore_body.is_empty(),
        "conflicting restore must retain the managed snapshot"
    );
}

#[test]
fn unified_tool_inventory_can_enable_codex_mcp_in_claude() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_config = temp.path().join(".claude.json");
    write(
        &codex_home.join("config.toml"),
        "[mcp_servers.codex-only]\ncommand = \"node\"\nargs = [\"server.js\"]\nenabled = true\n",
    );
    write(&claude_config, "{}");
    let roots = UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: vec![claude_config.clone()],
    };

    let inventory = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "codex-only".to_string(),
            kind: "mcp".to_string(),
            app: "claude".to_string(),
            enabled: true,
        },
    )
    .unwrap();

    let asset = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "codex-only")
        .unwrap();
    assert!(asset.codex.enabled);
    assert!(asset.claude.enabled);
    let claude_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(claude_config).unwrap()).unwrap();
    assert_eq!(claude_json["mcpServers"]["codex-only"]["command"], "node");
}

#[test]
fn unified_tool_inventory_plugin_toggles_are_independent_and_reversible() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    write(
        &codex_home.join("config.toml"),
        "[plugins.\"shared-plugin@local-market\"]\nenabled = true\n",
    );
    let codex_manifest =
        codex_home.join(".tmp/plugins/local-market/shared-plugin/.codex-plugin/plugin.json");
    write(
        &codex_manifest,
        r#"{"name":"shared-plugin","description":"shared plugin"}"#,
    );
    write(
        &claude_home.join("settings.json"),
        r#"{"enabledPlugins":{"shared-plugin@claude-plugins-official":true}}"#,
    );
    let claude_manifest = claude_home.join(
        "plugins/cache/claude-plugins-official/shared-plugin/1.0.0/.claude-plugin/plugin.json",
    );
    write(
        &claude_manifest,
        r#"{"name":"shared-plugin","description":"shared plugin"}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home: codex_home.clone(),
        claude_home: claude_home.clone(),
        claude_config_paths: Vec::new(),
    };

    let codex_off = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-plugin".to_string(),
            kind: "plugin".to_string(),
            app: "codex".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let asset = codex_off
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "shared-plugin")
        .unwrap();
    assert!(!asset.codex.enabled);
    assert!(asset.claude.enabled);
    assert!(codex_manifest.exists());
    assert!(claude_manifest.exists());

    let codex_on = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-plugin".to_string(),
            kind: "plugin".to_string(),
            app: "codex".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let asset = codex_on
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "shared-plugin")
        .unwrap();
    assert!(asset.codex.enabled);
    assert!(asset.claude.enabled);

    let claude_off = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-plugin".to_string(),
            kind: "plugin".to_string(),
            app: "claude".to_string(),
            enabled: false,
        },
    )
    .unwrap();
    let asset = claude_off
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "shared-plugin")
        .unwrap();
    assert!(asset.codex.enabled);
    assert!(!asset.claude.enabled);
    assert!(codex_manifest.exists());
    assert!(claude_manifest.exists());

    let claude_on = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "shared-plugin".to_string(),
            kind: "plugin".to_string(),
            app: "claude".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let asset = claude_on
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "shared-plugin")
        .unwrap();
    assert!(asset.codex.enabled);
    assert!(asset.claude.enabled);
}

#[test]
fn enabling_openai_cached_plugin_uses_the_manifest_marketplace_id() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    write(&codex_home.join("config.toml"), "model = \"gpt-test\"\n");
    write(
        &codex_home.join(".tmp/plugins/.agents/plugins/marketplace.json"),
        r#"{"name":"openai-curated","plugins":[{"name":"gmail"}]}"#,
    );
    write(
        &codex_home.join(".tmp/plugins/plugins/gmail/.codex-plugin/plugin.json"),
        r#"{"name":"gmail","description":"Gmail"}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home: codex_home.clone(),
        claude_home,
        claude_config_paths: Vec::new(),
    };

    let inventory = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "gmail".to_string(),
            kind: "plugin".to_string(),
            app: "codex".to_string(),
            enabled: true,
        },
    )
    .unwrap();

    let asset = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "gmail")
        .unwrap();
    assert!(asset.codex.enabled);
    let config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(config.contains("[plugins.\"gmail@openai-curated\"]"));
    assert!(!config.contains("gmail@plugins"));
}

#[test]
fn enabling_custom_local_marketplace_plugin_uses_its_manifest_id() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let marketplace_root = codex_home.join("custom-local-marketplace");
    write(
        &codex_home.join("config.toml"),
        "model = \"gpt-test\"\n\n[marketplaces.custom-local]\nsource_type = \"local\"\nsource = \"custom-local-marketplace\"\n",
    );
    write(
        &marketplace_root.join("marketplace.json"),
        r#"{"name":"custom-local","plugins":[{"name":"team-tool"}]}"#,
    );
    write(
        &marketplace_root.join("plugins/team-tool/.codex-plugin/plugin.json"),
        r#"{"name":"team-tool","description":"Team tool"}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home: codex_home.clone(),
        claude_home,
        claude_config_paths: Vec::new(),
    };

    let before = scan_unified_tool_inventory(&roots).unwrap();
    let cached = before
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "team-tool")
        .unwrap();
    assert!(cached.codex.available);
    assert!(cached.codex.toggle_supported);
    assert_eq!(cached.codex.config_id, "team-tool@custom-local");

    let inventory = set_unified_tool_asset_enabled(
        &roots,
        &UnifiedToolToggleRequest {
            id: "team-tool".to_string(),
            kind: "plugin".to_string(),
            app: "codex".to_string(),
            enabled: true,
        },
    )
    .unwrap();
    let asset = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "team-tool")
        .unwrap();
    assert!(asset.codex.enabled);
    let config = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(config.contains("[plugins.\"team-tool@custom-local\"]"));
    assert!(!config.contains("team-tool@plugins"));
}

#[test]
fn cached_plugin_without_a_marketplace_manifest_does_not_invent_an_id() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    write(&codex_home.join("config.toml"), "model = \"gpt-test\"\n");
    write(
        &codex_home.join(".tmp/plugins/plugins/orphan/.codex-plugin/plugin.json"),
        r#"{"name":"orphan","description":"Orphan cache"}"#,
    );
    let roots = UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: Vec::new(),
    };

    let inventory = scan_unified_tool_inventory(&roots).unwrap();
    let asset = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "orphan")
        .unwrap();

    assert!(asset.codex.available);
    assert!(!asset.codex.toggle_supported);
    assert!(asset.codex.config_id.is_empty());
}

#[test]
fn unified_tool_inventory_discovers_and_merges_both_apps() {
    let temp = tempdir().unwrap();
    let codex_home = temp.path().join("codex");
    let claude_home = temp.path().join("claude");
    let claude_config = temp.path().join("claude.json");

    write(
        &codex_home.join("config.toml"),
        r#"
[mcp_servers.shared-mcp]
command = "shared-server"
enabled = true

[mcp_servers.codex-only]
command = "codex-server"
enabled = true

[plugins."figma@openai-api-curated"]
enabled = true
"#,
    );
    write(
        &codex_home.join("skills/shared-skill/SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared description\n---\n",
    );
    write(
        &codex_home.join("skills/codex-only/SKILL.md"),
        "---\nname: codex-only\ndescription: Codex only\n---\n",
    );

    write(
        &claude_config,
        r#"{
  "mcpServers": {
    "shared-mcp": { "command": "shared-server", "enabled": true },
    "claude-only": { "command": "claude-server", "enabled": true }
  }
}"#,
    );
    write(
        &claude_home.join("skills/shared-skill/SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared description\n---\n",
    );
    write(
        &claude_home.join("skills/claude-only/SKILL.md"),
        "---\nname: claude-only\ndescription: Claude only\n---\n",
    );
    write(
        &claude_home.join("settings.json"),
        r#"{
  "enabledPlugins": {
    "figma@claude-plugins-official": true,
    "claude-plugin@claude-plugins-official": true
  }
}"#,
    );

    let roots = UnifiedToolInventoryRoots {
        codex_home,
        claude_home,
        claude_config_paths: vec![claude_config],
    };
    let inventory = scan_unified_tool_inventory(&roots).unwrap();

    assert_eq!(inventory.counts.mcp, 3);
    assert_eq!(inventory.counts.skills, 3);
    assert_eq!(inventory.counts.plugins, 2);

    let shared_mcp = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "mcp" && asset.id == "shared-mcp")
        .unwrap();
    assert!(shared_mcp.codex.enabled);
    assert!(shared_mcp.claude.enabled);

    let shared_skill = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "skill" && asset.id == "shared-skill")
        .unwrap();
    assert!(shared_skill.codex.enabled);
    assert!(shared_skill.claude.enabled);

    let figma = inventory
        .assets
        .iter()
        .find(|asset| asset.kind == "plugin" && asset.id == "figma")
        .unwrap();
    assert!(figma.codex.enabled);
    assert!(figma.claude.enabled);
    assert!(inventory.counts.raw_discoveries > inventory.counts.total);
    assert_eq!(
        inventory.counts.deduplicated,
        inventory.counts.raw_discoveries - inventory.counts.total
    );
}
