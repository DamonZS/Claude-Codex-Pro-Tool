import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  AppWindow,
  CheckCircle2,
  CircleDot,
  CircleOff,
  Languages,
  MessageCircle,
  RefreshCw,
  Settings2,
  ShieldCheck,
  SquareTerminal,
  Wrench,
  type LucideIcon,
} from "lucide-react";

import type { AgentScope } from "@/components/AppShell";
import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { compactDisplayPath, statusFailed, statusOk } from "@/lib/helpers";
import type {
  BackendSettings,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopResult,
  ClaudeZhPatchResult,
  OverviewResult,
  WatcherResult,
} from "@/types";

type ClientId = "codex" | "claude-desktop" | "claude-code";
type StateTone = "ok" | "attention" | "danger" | "muted";

type StateValue = {
  detail: string;
  label: string;
  tone: StateTone;
};

type Capability = {
  detail: string;
  enabled: boolean | null;
  label: string;
};

type ClientRecord = {
  capabilities: Capability[];
  details: Array<{ label: string; mono?: boolean; value: string }>;
  effective: StateValue;
  enabled: StateValue;
  health: StateValue;
  icon: LucideIcon;
  id: ClientId;
  installed: StateValue;
  label: string;
  subtitle: string;
};

type ClientsEnhancementScreenProps = {
  actions: AppActions;
  agentScope: AgentScope;
  claudeDesktop: ClaudeDesktopResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeZhPatch: ClaudeZhPatchResult | null;
  overview: OverviewResult | null;
  settings: BackendSettings | null;
  watcher: WatcherResult | null;
};

const UNKNOWN_STATE: StateValue = {
  detail: "刷新后读取本机状态",
  label: "未检测",
  tone: "muted",
};

function stateValue(label: string, detail: string, tone: StateTone): StateValue {
  return { detail, label, tone };
}

function stateIcon(tone: StateTone) {
  if (tone === "ok") return CheckCircle2;
  if (tone === "danger" || tone === "attention") return AlertTriangle;
  return CircleDot;
}

function StateCell({ label, value }: { label: string; value: StateValue }) {
  const Icon = stateIcon(value.tone);
  return (
    <div className={`client-state-cell ${value.tone}`}>
      <dt>{label}</dt>
      <dd>
        <span className="client-state-value">
          <Icon aria-hidden="true" className="h-4 w-4" />
          {value.label}
        </span>
        <small>{value.detail}</small>
      </dd>
    </div>
  );
}

function CapabilityState({ capability }: { capability: Capability }) {
  const enabled = capability.enabled === true;
  const unknown = capability.enabled === null;
  const Icon = enabled ? CheckCircle2 : unknown ? CircleDot : CircleOff;
  return (
    <li>
      <span className={`client-capability-icon ${enabled ? "enabled" : unknown ? "unknown" : "disabled"}`}>
        <Icon aria-hidden="true" className="h-4 w-4" />
      </span>
      <span>
        <strong>{capability.label}</strong>
        <small>{capability.detail}</small>
      </span>
      <span className={`client-capability-label ${enabled ? "enabled" : unknown ? "unknown" : "disabled"}`}>
        {enabled ? "已启用" : unknown ? "未检测" : "未启用"}
      </span>
    </li>
  );
}

function buildClientRecords({
  claudeDesktop,
  claudeDesktopDevMode,
  claudeZhPatch,
  overview,
  settings,
  watcher,
}: Omit<ClientsEnhancementScreenProps, "actions" | "agentScope">): ClientRecord[] {
  const launch = overview?.latest_launch;
  const codexInstalled = Boolean(overview?.codex_app.path) || statusOk(overview?.codex_app.status);
  const codexRunning = launch?.status === "running" || launch?.status === "degraded";
  const codexFrontendOnline = Boolean(launch?.frontend_runtime_online || launch?.debug_port_online);
  const codexBackendOnline = Boolean(launch?.helper_port_online);
  const codexFailed = Boolean(
    (overview && statusFailed(overview.status))
    || statusFailed(overview?.codex_app.status)
    || statusFailed(launch?.status),
  );
  const codexEnhancementsEnabled = settings?.enhancementsEnabled ?? null;
  const codexEffective = codexRunning && codexFrontendOnline && codexBackendOnline && codexEnhancementsEnabled === true;
  const codexPartiallyEffective = codexRunning && (codexFrontendOnline || codexBackendOnline);

  const patchInstalled = Boolean(
    claudeZhPatch?.status.localeConfigured
    && claudeZhPatch.status.frontendI18nPresent
    && claudeZhPatch.status.chunkPatchPresent,
  );
  const devModeConfigured = claudeDesktopDevMode?.devModeStatus.configured ?? false;
  const claudeInstalled = Boolean(
    claudeDesktop?.executablePaths.length
    || (claudeDesktop?.installKind && !["unknown", "not_found", "none"].includes(claudeDesktop.installKind)),
  );
  const claudeRunning = (claudeDesktop?.processCount ?? 0) > 0;
  const claudeInspectorReady = Boolean(
    claudeDesktop?.inspectorPorts.length
    || claudeDesktop?.cdpStatus === "node_inspector_ready"
    || claudeDesktop?.cdpStatus === "ok",
  );
  const claudeFailed = Boolean(
    (claudeDesktop && statusFailed(claudeDesktop.status))
    || statusFailed(claudeDesktop?.integrityStatus)
    || claudeZhPatch?.status.status === "failed"
    || (claudeDesktopDevMode && statusFailed(claudeDesktopDevMode.status)),
  );
  const claudeEnhancementsEnabled = settings
    ? Boolean(settings.claudeAppChineseOverlayEnabled || patchInstalled || devModeConfigured)
    : claudeDesktopDevMode || claudeZhPatch
      ? Boolean(patchInstalled || devModeConfigured)
      : null;
  const claudeEnhancementConfigured = Boolean(patchInstalled || devModeConfigured);
  const activeClaudeDesktopProfile = settings?.relayProfiles.find(
    (profile) => profile.id === settings.activeClaudeDesktopRelayId,
  );

  const cliWrapperEnabled = settings?.cliWrapperEnabled ?? null;
  const watcherEnabled = watcher?.enabled ?? null;
  const activeClaudeCodeProfile = settings?.relayProfiles.find(
    (profile) => profile.id === settings.activeClaudeRelayId,
  );
  const claudeCodePartiallyEffective = cliWrapperEnabled === true || watcherEnabled === true;
  const claudeCodeEffective = cliWrapperEnabled === true && watcherEnabled === true;
  const watcherFailed = Boolean(watcher && statusFailed(watcher.status));

  return [
    {
      id: "codex",
      label: "Codex App",
      subtitle: "本地启动、窗口增强与代理运行时",
      icon: AppWindow,
      installed: overview
        ? codexFailed
          ? stateValue("检测异常", "无法确认 Codex 安装状态", "danger")
          : codexInstalled
            ? stateValue("已安装", compactDisplayPath(overview.codex_app.path), "ok")
            : stateValue("未检测到", "可在维护页选择应用路径", "attention")
        : UNKNOWN_STATE,
      enabled: settings
        ? codexEnhancementsEnabled
          ? stateValue("已启用", "应用增强总开关已开启", "ok")
          : stateValue("未启用", "当前使用基础启动能力", "muted")
        : UNKNOWN_STATE,
      effective: launch
        ? codexEffective
          ? stateValue("当前生效", "前端注入与本地后端均在线", "ok")
          : codexPartiallyEffective
            ? stateValue("部分生效", "运行时仍有组件需要检查", "attention")
            : stateValue("未生效", "Codex 未运行或增强未连接", "muted")
        : UNKNOWN_STATE,
      health: overview
        ? codexFailed
          ? stateValue("运行异常", launch?.message || overview.message, "danger")
          : codexRunning && codexFrontendOnline && codexBackendOnline
            ? stateValue("健康", "前端与后端连接正常", "ok")
            : codexRunning
              ? stateValue("需要检查", "运行中，但连接不完整", "attention")
              : stateValue("未运行", "启动后可检查实时健康状态", "muted")
        : UNKNOWN_STATE,
      capabilities: [
        {
          label: "应用增强总开关",
          detail: "控制 Codex 本地窗口增强能力",
          enabled: codexEnhancementsEnabled,
        },
        {
          label: "模型与服务层级控制",
          detail: "扩展模型白名单与服务层级选择",
          enabled: settings ? settings.codexAppModelWhitelistUnlock || settings.codexAppServiceTierControls : null,
        },
        {
          label: "插件市场入口",
          detail: "显示并维护 Codex 插件市场入口",
          enabled: settings?.codexAppPluginMarketplaceUnlock ?? null,
        },
        {
          label: "会话操作增强",
          detail: "删除、导出、迁移与时间线能力",
          enabled: settings
            ? settings.codexAppSessionDelete
              || settings.codexAppMarkdownExport
              || settings.codexAppProjectMove
              || settings.codexAppConversationTimeline
            : null,
        },
      ],
      details: [
        { label: "应用版本", value: overview?.codex_version || "未检测" },
        { label: "启动模式", value: settings?.launchMode === "relay" ? "本地代理" : settings?.launchMode === "patch" ? "本地增强" : "未读取" },
        { label: "前端运行时", value: codexFrontendOnline ? "在线" : "离线" },
        { label: "本地后端", value: codexBackendOnline ? "在线" : "离线" },
      ],
    },
    {
      id: "claude-desktop",
      label: "Claude Desktop",
      subtitle: "本机汉化、开发模式与供应商配置",
      icon: MessageCircle,
      installed: claudeDesktop
        ? claudeFailed && !claudeInstalled
          ? stateValue("检测异常", "无法确认 Claude Desktop 安装", "danger")
          : claudeInstalled
            ? stateValue("已安装", claudeDesktop.installKind || "本机安装", "ok")
            : stateValue("未检测到", "未发现可执行文件", "attention")
        : UNKNOWN_STATE,
      enabled: claudeEnhancementsEnabled === null
        ? UNKNOWN_STATE
        : claudeEnhancementsEnabled
          ? stateValue("已启用", "至少一项本地增强已配置", "ok")
          : stateValue("未启用", "尚未配置汉化或开发模式", "muted"),
      effective: claudeDesktop
        ? claudeRunning && claudeEnhancementConfigured
          ? stateValue(
            "待重启确认",
            patchInstalled ? "汉化资源或开发配置已写入，需重启后实际确认" : "开发配置已写入，需重启后实际确认",
            "attention",
          )
          : claudeRunning
            ? stateValue("原生运行", "客户端运行中，增强尚未确认生效", "attention")
            : claudeEnhancementConfigured
              ? stateValue("待启动验证", "本地配置已写入，启动客户端后验证", "attention")
              : stateValue("未运行", "启动后确认本地增强状态", "muted")
        : UNKNOWN_STATE,
      health: claudeDesktop
        ? claudeFailed
          ? stateValue("需要修复", claudeDesktop.integrityMessage || claudeDesktop.message, "danger")
          : claudeRunning && claudeInspectorReady
            ? stateValue("健康", "客户端与 Inspector 状态正常", "ok")
            : claudeRunning
              ? stateValue("运行中", "Inspector 尚未就绪", "attention")
              : stateValue("未运行", "未发现 Claude Desktop 进程", "muted")
        : UNKNOWN_STATE,
      capabilities: [
        {
          label: "本机汉化资源",
          detail: patchInstalled ? "Locale、前端资源与 Chunk 已写入" : "尚未检测到完整汉化资源",
          enabled: claudeZhPatch ? patchInstalled : null,
        },
        {
          label: "开发模式配置",
          detail: devModeConfigured ? "本地开发配置已写入" : "尚未写入开发配置",
          enabled: claudeDesktopDevMode ? devModeConfigured : null,
        },
        {
          label: "第三方供应商配置",
          detail: activeClaudeDesktopProfile
            ? `当前配置：${activeClaudeDesktopProfile.name || activeClaudeDesktopProfile.id}`
            : "当前未选择第三方供应商",
          enabled: settings ? Boolean(activeClaudeDesktopProfile) : null,
        },
        {
          label: "中文覆盖层",
          detail: "仅表示本地界面增强开关，不涉及账号状态",
          enabled: settings?.claudeAppChineseOverlayEnabled ?? null,
        },
      ],
      details: [
        { label: "安装类型", value: claudeDesktop?.installKind || "未检测" },
        { label: "运行进程", value: claudeDesktop ? `${claudeDesktop.processCount} 个` : "未检测" },
        { label: "CDP / Inspector", value: claudeDesktop?.cdpStatus || "未检测" },
        { label: "完整性", value: claudeDesktop?.integrityStatus || "未检测" },
      ],
    },
    {
      id: "claude-code",
      label: "Claude Code",
      subtitle: "CLI 包装、第三方供应商配置与 Watcher",
      icon: SquareTerminal,
      installed: stateValue("未单独检测", "当前接口不读取官方 CLI 安装状态", "muted"),
      enabled: cliWrapperEnabled === null
        ? UNKNOWN_STATE
        : cliWrapperEnabled
          ? stateValue("已启用", "CLI 包装入口已开启", "ok")
          : stateValue("未启用", "CLI 包装入口已关闭", "muted"),
      effective: cliWrapperEnabled === null && watcherEnabled === null
        ? UNKNOWN_STATE
        : claudeCodeEffective
          ? stateValue("待使用验证", "CLI 包装与 Watcher 已启用，需在下次启动验证", "attention")
          : claudeCodePartiallyEffective
            ? stateValue("配置不完整", "CLI 包装与 Watcher 状态不一致", "attention")
            : stateValue("未配置", "本地配置能力未启用", "muted"),
      health: watcher
        ? watcherFailed
          ? stateValue("状态异常", watcher.message, "danger")
          : claudeCodeEffective
            ? stateValue("配置就绪", "Watcher 与 CLI 包装均已启用", "ok")
            : watcher.enabled
              ? stateValue("等待配置", "Watcher 已启用，CLI 包装尚未开启", "attention")
              : stateValue("未启用", "Watcher 当前未启用", "muted")
        : UNKNOWN_STATE,
      capabilities: [
        {
          label: "CLI 包装入口",
          detail: "为 Claude Code 应用本地供应商与协议配置",
          enabled: cliWrapperEnabled,
        },
        {
          label: "Watcher 配置监测",
          detail: "监测本地配置状态并执行既有维护规则",
          enabled: watcherEnabled,
        },
        {
          label: "第三方供应商配置",
          detail: activeClaudeCodeProfile
            ? `当前配置：${activeClaudeCodeProfile.name || activeClaudeCodeProfile.id}`
            : "当前未选择第三方供应商",
          enabled: settings ? Boolean(activeClaudeCodeProfile) : null,
        },
        {
          label: "供应商配置集",
          detail: settings ? `${settings.relayProfiles.filter((profile) => profile.targetApp === "claude").length} 个 Claude Code 配置` : "尚未读取",
          enabled: settings ? settings.relayProfiles.some((profile) => profile.targetApp === "claude") : null,
        },
      ],
      details: [
        { label: "当前供应商", value: activeClaudeCodeProfile?.name || activeClaudeCodeProfile?.id || "未选择" },
        { label: "CLI 包装", value: cliWrapperEnabled === null ? "未检测" : cliWrapperEnabled ? "已启用" : "未启用" },
        { label: "Watcher", value: watcherEnabled === null ? "未检测" : watcherEnabled ? "已启用" : "未启用" },
        { label: "配置来源", value: "本地管理工具" },
      ],
    },
  ];
}

function clientMatchesScope(client: ClientRecord, agentScope: AgentScope) {
  if (agentScope === "all") return true;
  return agentScope === "codex" ? client.id === "codex" : client.id !== "codex";
}

export function ClientsEnhancementScreen(props: ClientsEnhancementScreenProps) {
  const {
    actions,
    agentScope,
    claudeDesktop,
    claudeDesktopDevMode,
    claudeZhPatch,
    overview,
    settings,
    watcher,
  } = props;
  const clients = useMemo(
    () => buildClientRecords({ claudeDesktop, claudeDesktopDevMode, claudeZhPatch, overview, settings, watcher }),
    [claudeDesktop, claudeDesktopDevMode, claudeZhPatch, overview, settings, watcher],
  );
  const visibleClients = useMemo(
    () => clients.filter((client) => clientMatchesScope(client, agentScope)),
    [agentScope, clients],
  );
  const [selectedId, setSelectedId] = useState<ClientId>("codex");

  useEffect(() => {
    if (!visibleClients.some((client) => client.id === selectedId) && visibleClients[0]) {
      setSelectedId(visibleClients[0].id);
    }
  }, [selectedId, visibleClients]);

  const selected = visibleClients.find((client) => client.id === selectedId) ?? visibleClients[0];
  const noStateLoaded = !overview && !claudeDesktop && !settings && !watcher;
  const failedSources = [
    overview && statusFailed(overview.status) ? "Codex" : null,
    claudeDesktop && statusFailed(claudeDesktop.status) ? "Claude Desktop" : null,
    claudeDesktopDevMode && statusFailed(claudeDesktopDevMode.status) ? "Claude 开发模式" : null,
    claudeZhPatch?.status.status === "failed" ? "Claude 汉化" : null,
    watcher && statusFailed(watcher.status) ? "Watcher" : null,
  ].filter((value): value is string => Boolean(value));

  return (
    <div className="clients-enhancement-screen">
      {noStateLoaded ? (
        <div className="clients-state-banner loading" role="status">
          <RefreshCw aria-hidden="true" className="h-4 w-4 spin" />
          <span>正在读取本机客户端与增强状态...</span>
        </div>
      ) : null}

      {failedSources.length ? (
        <div className="clients-state-banner danger" role="alert">
          <AlertTriangle aria-hidden="true" className="h-4 w-4" />
          <span>{failedSources.join("、")} 状态读取失败，页面保留已取得的数据；可刷新后重试。</span>
        </div>
      ) : null}

      <section aria-label="客户端与增强状态" className="clients-console">
        <aside className="clients-master-list">
          <header>
            <strong>本机客户端</strong>
            <span>{visibleClients.length} 个当前范围</span>
          </header>
          <div className="clients-master-items">
            {visibleClients.map((client) => {
              const Icon = client.icon;
              const HealthIcon = stateIcon(client.health.tone);
              return (
                <button
                  aria-pressed={selected?.id === client.id}
                  className={selected?.id === client.id ? "active" : ""}
                  key={client.id}
                  onClick={() => setSelectedId(client.id)}
                  type="button"
                >
                  <span className="client-product-icon"><Icon aria-hidden="true" className="h-5 w-5" /></span>
                  <span className="client-product-copy">
                    <strong>{client.label}</strong>
                    <small>{client.subtitle}</small>
                  </span>
                  <span className={`client-list-health ${client.health.tone}`} title={`健康状态：${client.health.label}`}>
                    <HealthIcon aria-hidden="true" className="h-4 w-4" />
                    <span className="sr-only">{client.health.label}</span>
                  </span>
                </button>
              );
            })}
          </div>
        </aside>

        {selected ? (
          <article className="client-inspector">
            <header className="client-inspector-header">
              <span className="client-product-icon large"><selected.icon aria-hidden="true" className="h-6 w-6" /></span>
              <span>
                <strong>{selected.label}</strong>
                <small>{selected.subtitle}</small>
              </span>
            </header>

            <dl className="client-state-grid">
              <StateCell label="已安装" value={selected.installed} />
              <StateCell label="已启用" value={selected.enabled} />
              <StateCell label="当前生效" value={selected.effective} />
              <StateCell label="健康状态" value={selected.health} />
            </dl>

            <div className="client-inspector-body">
              <section className="client-capabilities" aria-labelledby="client-capabilities-title">
                <header>
                  <div>
                    <strong id="client-capabilities-title">增强能力</strong>
                    <small>本机配置与实时状态分开显示</small>
                  </div>
                  <ShieldCheck aria-hidden="true" className="h-4 w-4" />
                </header>
                <ul>
                  {selected.capabilities.map((capability) => (
                    <CapabilityState capability={capability} key={capability.label} />
                  ))}
                </ul>
              </section>

              <section className="client-runtime-details" aria-labelledby="client-runtime-title">
                <header>
                  <div>
                    <strong id="client-runtime-title">运行详情</strong>
                    <small>只显示本地检测结果</small>
                  </div>
                  <Settings2 aria-hidden="true" className="h-4 w-4" />
                </header>
                <dl>
                  {selected.details.map((detail) => (
                    <div key={detail.label}>
                      <dt>{detail.label}</dt>
                      <dd className={detail.mono ? "font-mono" : undefined}>{detail.value}</dd>
                    </div>
                  ))}
                </dl>
              </section>
            </div>

            <footer className="client-action-bar">
              {selected.id === "codex" ? (
                <>
                  <Button onClick={() => void actions.restartCodex()}>
                    <RefreshCw aria-hidden="true" className="h-4 w-4" />
                    启动/重启 Codex
                  </Button>
                  <Button onClick={() => void actions.repairFrontendConnection()} variant="outline">
                    <Wrench aria-hidden="true" className="h-4 w-4" />
                    修复连接
                  </Button>
                </>
              ) : null}
              {selected.id === "claude-desktop" ? (
                <>
                  <Button onClick={() => void actions.launchClaudeDesktop()}>
                    <MessageCircle aria-hidden="true" className="h-4 w-4" />
                    启动/重启 Claude
                  </Button>
                  <Button onClick={() => void actions.installClaudeZhPatch()} variant="outline">
                    <Languages aria-hidden="true" className="h-4 w-4" />
                    一键汉化
                  </Button>
                  <Button onClick={() => void actions.configureClaudeDesktopDevMode()} variant="outline">
                    <Wrench aria-hidden="true" className="h-4 w-4" />
                    开发模式
                  </Button>
                </>
              ) : null}
              {selected.id === "claude-code" ? (
                <>
                  <Button onClick={() => void actions.installWatcher()}>
                    <SquareTerminal aria-hidden="true" className="h-4 w-4" />
                    安装 Watcher
                  </Button>
                  {watcher?.enabled ? (
                    <Button onClick={() => void actions.disableWatcher()} variant="outline">停用 Watcher</Button>
                  ) : (
                    <Button onClick={() => void actions.enableWatcher()} variant="outline">启用 Watcher</Button>
                  )}
                </>
              ) : null}
            </footer>
          </article>
        ) : (
          <div className="client-empty-state" role="status">
            当前 Agent 范围没有可显示的客户端。
          </div>
        )}
      </section>
    </div>
  );
}
