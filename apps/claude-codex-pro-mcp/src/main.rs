//! 盘古记忆 MCP server（阶段 4 / 模块 D）。
//!
//! 一个独立的 stdio MCP server，把盘古记忆的读写能力暴露给任意支持 MCP 的
//! agent（Claude Code / Cursor / Codex CLI 等）。它读取与 Codex 注入侧 HTTP
//! bridge **同一份** `memory_assist.sqlite` 和 **同一份** `SettingsStore`
//! 设置文件，因此天然共享同一份大脑与全部门控，无需任何额外 IPC。
//!
//! 详见 `docs/adr/0002-pangu-memory-mcp-cross-agent.md`。
//!
//! 门控（双层）：
//! - 启动门控：`memoryAssistMcpEnabled` 关闭时 server 直接退出，不提供服务。
//! - 每工具复查：每次调用复查 `memoryAssistEnabled`（总开关），写工具
//!   `memory_learn` 受总开关约束；关闭时返回明确错误，绝不静默写库。

use claude_codex_pro_core::memory_assist::{
    MemoryAssistStore, MemoryItem, MemoryItemRequest, MemoryQueryRequest, MemoryQueryResult,
};
use claude_codex_pro_core::settings::SettingsStore;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ErrorData, ServerCapabilities, ServerInfo};
use rmcp::{ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MCP 工具默认返回条数（客户端可用 `limit` 覆盖）。
const DEFAULT_LIMIT: usize = 12;
/// Hard cap for caller-controlled result sets to bound query and serialization work.
const MAX_LIMIT: usize = 100;

/// 把任意错误转成 MCP 错误响应。
fn internal_error(message: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(message.into(), None)
}

/// 读取当前设置。读失败按“未启用”处理（安全默认：宁可不暴露）。
fn load_settings() -> claude_codex_pro_core::settings::BackendSettings {
    SettingsStore::default().load().unwrap_or_default()
}

/// 每工具复查总开关：`memoryAssistEnabled` 关闭时拒绝任何读写。
fn ensure_memory_enabled() -> Result<(), ErrorData> {
    if load_settings().memory_assist_enabled {
        Ok(())
    } else {
        Err(internal_error(
            "盘古记忆已禁用（memoryAssistEnabled=false）。",
        ))
    }
}

/// `MemoryItem` 的 MCP 精简视图：只暴露 agent 需要的字段，隐藏内部计时/强度细节。
#[derive(Debug, Serialize, JsonSchema)]
struct MemoryView {
    id: String,
    text: String,
    workspace: String,
    category: String,
    tags: Vec<String>,
    source: String,
}

impl From<MemoryItem> for MemoryView {
    fn from(item: MemoryItem) -> Self {
        Self {
            id: item.id,
            text: item.text,
            workspace: item.workspace,
            category: item.category,
            tags: item.tags,
            source: item.source,
        }
    }
}

/// 归一化 workspace 参数：缺省或空白落到 `global`（跨 agent 默认共享同一份大脑，
/// 见 ADR 0002 决策 4）。客户端可显式传约定键（如 `agent://<id>/<repo>`）来隔离。
fn workspace_or_global(workspace: Option<String>) -> String {
    match workspace {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => "global".to_string(),
    }
}

fn clamp_limit(limit: Option<usize>) -> usize {
    limit
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_LIMIT)
        .min(MAX_LIMIT)
}

fn search_with_mcp_activity(
    store: &MemoryAssistStore,
    query: String,
    workspace: String,
    limit: usize,
) -> anyhow::Result<MemoryQueryResult> {
    store.query_with_activity(
        MemoryQueryRequest {
            query,
            workspace,
            include_global: true,
            include_archived: false,
            limit,
        },
        "mcp",
        "search",
        None,
    )
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchParams {
    /// 检索关键词 / 自然语言查询。
    query: String,
    /// 目标 workspace；缺省用 `global`（跨 agent 共享）。可传 `agent://<id>/<repo>` 隔离。
    #[serde(default)]
    workspace: Option<String>,
    /// 返回条数上限，默认 12，最大 100。
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListParams {
    /// 目标 workspace；缺省用 `global`。
    #[serde(default)]
    workspace: Option<String>,
    /// 返回条数上限，默认 12，最大 100。
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RecentParams {
    /// 目标 workspace；缺省用 `global`。
    #[serde(default)]
    workspace: Option<String>,
    /// 返回条数上限，默认 12，最大 100。
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LearnParams {
    /// 要长期记住的经验教训 / 事实 / 约定。
    text: String,
    /// 目标 workspace；缺省用 `global`。
    #[serde(default)]
    workspace: Option<String>,
    /// 分类标签（如 lesson-learned / project-rule）；缺省 general。
    #[serde(default)]
    category: Option<String>,
    /// 记忆来源标识；缺省 mcp。
    #[serde(default)]
    source: Option<String>,
}

/// 盘古记忆 MCP server。持有一个 `MemoryAssistStore`（默认指向共享 sqlite）。
#[derive(Clone)]
struct PanguMemoryServer {
    store: MemoryAssistStore,
    // The `#[tool_router]` / `#[tool_handler]` macros read this field in their
    // generated code, but dead-code analysis can't see that indirection.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl PanguMemoryServer {
    fn new() -> Self {
        Self {
            store: MemoryAssistStore::default(),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "在盘古记忆中检索相关经验教训（语义 + 关键词混合排序）。返回匹配的记忆条目。"
    )]
    async fn memory_search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<rmcp::Json<Vec<MemoryView>>, ErrorData> {
        ensure_memory_enabled()?;
        let workspace = workspace_or_global(params.workspace);
        let limit = clamp_limit(params.limit);
        let store = self.store.clone();
        let result = tokio::task::spawn_blocking(move || {
            search_with_mcp_activity(&store, params.query, workspace, limit)
        })
        .await
        .map_err(|error| internal_error(format!("检索任务失败：{error}")))?
        .map_err(|error| internal_error(format!("检索失败：{error}")))?;
        let views = result
            .results
            .into_iter()
            .map(|scored| MemoryView::from(scored.item))
            .collect();
        Ok(rmcp::Json(views))
    }

    #[tool(description = "列出某个 workspace 的盘古记忆条目（按更新时间倒序）。")]
    async fn memory_list(
        &self,
        Parameters(params): Parameters<ListParams>,
    ) -> Result<rmcp::Json<Vec<MemoryView>>, ErrorData> {
        ensure_memory_enabled()?;
        let workspace = workspace_or_global(params.workspace);
        let limit = clamp_limit(params.limit);
        let store = self.store.clone();
        let items = tokio::task::spawn_blocking(move || {
            store.list_items(MemoryQueryRequest {
                query: String::new(),
                workspace,
                include_global: true,
                include_archived: false,
                limit,
            })
        })
        .await
        .map_err(|error| internal_error(format!("列表任务失败：{error}")))?
        .map_err(|error| internal_error(format!("列表失败：{error}")))?;
        Ok(rmcp::Json(
            items.into_iter().map(MemoryView::from).collect(),
        ))
    }

    #[tool(description = "获取最近更新的盘古记忆条目（跨 workspace 的近期记忆，按更新时间倒序）。")]
    async fn memory_recent(
        &self,
        Parameters(params): Parameters<RecentParams>,
    ) -> Result<rmcp::Json<Vec<MemoryView>>, ErrorData> {
        ensure_memory_enabled()?;
        let workspace = workspace_or_global(params.workspace);
        let limit = clamp_limit(params.limit);
        let store = self.store.clone();
        // list_items 空 query 已按 updated_at DESC 排序，即“最近”。
        let items = tokio::task::spawn_blocking(move || {
            store.list_items(MemoryQueryRequest {
                query: String::new(),
                workspace,
                include_global: true,
                include_archived: false,
                limit,
            })
        })
        .await
        .map_err(|error| internal_error(format!("最近记忆任务失败：{error}")))?
        .map_err(|error| internal_error(format!("最近记忆失败：{error}")))?;
        Ok(rmcp::Json(
            items.into_iter().map(MemoryView::from).collect(),
        ))
    }

    #[tool(
        description = "向盘古记忆写入一条经验教训 / 事实 / 约定（受 memoryAssistEnabled 总开关约束）。"
    )]
    async fn memory_learn(
        &self,
        Parameters(params): Parameters<LearnParams>,
    ) -> Result<rmcp::Json<MemoryView>, ErrorData> {
        ensure_memory_enabled()?;
        let text = params.text.trim().to_string();
        if text.is_empty() {
            return Err(internal_error("记忆内容不能为空。"));
        }
        let workspace = workspace_or_global(params.workspace);
        let category = params
            .category
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "general".to_string());
        let source = params
            .source
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "mcp".to_string());
        let store = self.store.clone();
        let item = tokio::task::spawn_blocking(move || {
            store.learn_item(MemoryItemRequest {
                text,
                workspace,
                category,
                tags: Vec::new(),
                source,
                source_session_id: String::new(),
            })
        })
        .await
        .map_err(|error| internal_error(format!("写入任务失败：{error}")))?
        .map_err(|error| internal_error(format!("写入失败：{error}")))?;
        Ok(rmcp::Json(MemoryView::from(item)))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for PanguMemoryServer {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo is #[non_exhaustive]; build from Default and set fields.
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "盘古记忆 MCP server：跨 agent 共享的本地记忆大脑。工具：memory_search / \
             memory_list / memory_recent（读）、memory_learn（写）。workspace 参数缺省为 \
             global（跨 agent 共享），可传 agent://<id>/<repo> 隔离。"
                .to_string(),
        );
        info
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 启动门控：MCP 开关关闭时直接退出，不对外暴露任何记忆（ADR 0002 决策 6）。
    if !load_settings().memory_assist_mcp_enabled {
        eprintln!(
            "盘古记忆 MCP server 未启用（memoryAssistMcpEnabled=false）。请在管理器设置中开启后重试。"
        );
        return Ok(());
    }

    let transport = rmcp::transport::stdio();
    let service = PanguMemoryServer::new()
        .serve(transport)
        .await
        .map_err(|error| anyhow::anyhow!("启动盘古记忆 MCP server 失败：{error}"))?;
    service
        .waiting()
        .await
        .map_err(|error| anyhow::anyhow!("盘古记忆 MCP server 运行中断：{error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_or_global_defaults_blank_to_global_and_trims() {
        // 缺省 / 空白 → global（跨 agent 默认共享同一份大脑，ADR 0002 决策 4）。
        assert_eq!(workspace_or_global(None), "global");
        assert_eq!(workspace_or_global(Some("   ".to_string())), "global");
        assert_eq!(workspace_or_global(Some(String::new())), "global");
        // 显式键被保留并 trim，既兼容旧 codex: 键，也容纳新 agent:// 约定键。
        assert_eq!(
            workspace_or_global(Some("codex:repo:a".to_string())),
            "codex:repo:a"
        );
        assert_eq!(
            workspace_or_global(Some("  agent://claude/repo-x  ".to_string())),
            "agent://claude/repo-x"
        );
    }

    #[test]
    fn clamp_limit_falls_back_to_default_and_rejects_zero() {
        assert_eq!(clamp_limit(None), DEFAULT_LIMIT);
        assert_eq!(clamp_limit(Some(0)), DEFAULT_LIMIT);
        assert_eq!(clamp_limit(Some(5)), 5);
        assert_eq!(clamp_limit(Some(MAX_LIMIT)), MAX_LIMIT);
        assert_eq!(clamp_limit(Some(MAX_LIMIT + 1)), MAX_LIMIT);
        assert_eq!(clamp_limit(Some(usize::MAX)), MAX_LIMIT);
    }

    #[test]
    fn memory_view_hides_internal_tier_and_strength_fields() {
        // MCP 视图只暴露 agent 需要的字段，隐藏内部计时/强度/归档细节。
        let item = MemoryItem {
            id: "mem-1".to_string(),
            text: "发布前先在外置磁盘备份源码".to_string(),
            workspace: "codex:repo:a".to_string(),
            category: "lesson-learned".to_string(),
            tags: vec!["backup".to_string()],
            source: "mcp".to_string(),
            source_session_id: "s1".to_string(),
            created_at: 1,
            updated_at: 2,
            last_accessed_at: 3,
            access_count: 4,
            tier: "active".to_string(),
            strength: 2.5,
            archived_at: 0,
            retention: 0.9,
            exempt: false,
        };
        let view = MemoryView::from(item);
        assert_eq!(view.id, "mem-1");
        assert_eq!(view.workspace, "codex:repo:a");
        assert_eq!(view.source, "mcp");
        let json = serde_json::to_value(&view).unwrap();
        // 内部字段不得出现在 MCP 输出里。
        assert!(json.get("tier").is_none());
        assert!(json.get("strength").is_none());
        assert!(json.get("retention").is_none());
        assert!(json.get("lastAccessedAt").is_none());
        assert!(json.get("last_accessed_at").is_none());
    }

    #[test]
    fn store_learn_and_query_round_trip_across_old_and_new_workspace_keys() {
        // acceptance #18：新旧 workspace 键共用同一张表、同一套语义。
        // 旧 codex: 键与新 agent:// 键都能 learn 并被检索到。
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));

        let codex_key = workspace_or_global(Some("codex:repo:a".to_string()));
        store
            .learn_item(MemoryItemRequest {
                text: "在 codex 工作区学到的构建经验：改完前端必须重新构建".to_string(),
                workspace: codex_key.clone(),
                category: "lesson-learned".to_string(),
                tags: Vec::new(),
                source: "mcp".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn codex-key item");

        let agent_key = workspace_or_global(Some("agent://claude/repo-a".to_string()));
        store
            .learn_item(MemoryItemRequest {
                text: "在 agent 工作区学到的切换经验：换供应商要同步历史会话".to_string(),
                workspace: agent_key.clone(),
                category: "lesson-learned".to_string(),
                tags: Vec::new(),
                source: "mcp".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn agent-key item");

        // 各自 workspace 能查到自己的条目。
        let codex_hits = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: codex_key.clone(),
                include_global: false,
                include_archived: false,
                limit: 20,
            })
            .expect("list codex-key");
        assert!(codex_hits.iter().any(|it| it.workspace == codex_key));

        let agent_hits = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: agent_key.clone(),
                include_global: false,
                include_archived: false,
                limit: 20,
            })
            .expect("list agent-key");
        assert!(agent_hits.iter().any(|it| it.workspace == agent_key));

        // __all__ 能同时看到新旧两种键的记忆。
        let all = store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: "__all__".to_string(),
                include_global: true,
                include_archived: false,
                limit: 50,
            })
            .expect("list all");
        assert!(all.iter().any(|it| it.workspace == codex_key));
        assert!(all.iter().any(|it| it.workspace == agent_key));
    }

    #[test]
    fn memory_search_records_mcp_source_while_plain_list_records_no_recall() {
        let temp = tempfile::tempdir().unwrap();
        let store = MemoryAssistStore::new(temp.path().join("memory_assist.sqlite"));
        store
            .learn_item(MemoryItemRequest {
                text: "MCP 搜索命中后必须留下真实召回来源".to_string(),
                workspace: "repo-mcp".to_string(),
                category: "project-rule".to_string(),
                tags: Vec::new(),
                source: "test".to_string(),
                source_session_id: String::new(),
            })
            .expect("learn MCP fixture");

        store
            .list_items(MemoryQueryRequest {
                query: String::new(),
                workspace: "repo-mcp".to_string(),
                include_global: true,
                include_archived: false,
                limit: 10,
            })
            .expect("plain list");
        assert_eq!(
            store
                .outcome_dashboard("repo-mcp", 7)
                .expect("dashboard after list")
                .today_recalls,
            0
        );

        let result = search_with_mcp_activity(
            &store,
            "MCP 搜索 真实召回来源".to_string(),
            "repo-mcp".to_string(),
            10,
        )
        .expect("MCP sourced search");
        assert_eq!(result.results.len(), 1);
        let dashboard = store
            .outcome_dashboard("repo-mcp", 7)
            .expect("dashboard after search");
        assert_eq!(dashboard.today_recalls, 1);
        assert_eq!(dashboard.recent_recalls[0].agent, "mcp");
        assert_eq!(dashboard.recent_recalls[0].event_type, "search");
        assert_eq!(dashboard.recent_recalls[0].workspace, "repo-mcp");
        assert!(dashboard.recent_recalls[0].memory.is_some());
    }
}
