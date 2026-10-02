import { useEffect, useMemo, useState } from "react";
import { Activity, Globe2, Plus, RotateCcw, Server, Trash2, Zap } from "lucide-react";

import { useI18n } from "@/lib/i18n";
import type { MessageKey } from "@/lib/locales/zh";
import { invokeCommand } from "@/tauriBridge";

import {
  SETTINGS_COMMANDS,
  type AgentAppId,
  type AppProxyConfig,
  type ProxyAppId,
  type ProxyScanResult,
  type ProxyTestResult,
  type RoutingConfig,
  type RoutingResult,
} from "./contract";
import { NumberField, SettingsSection, Switch, ToggleRow } from "./SettingsControls";

type Notify = (tone: "ok" | "error", text: string) => void;
export type ProviderOption = { id: string; name: string; appId: AgentAppId };

const APP_LABELS: Record<ProxyAppId, string> = { codex: "Codex", claude: "Claude Code", "claude-desktop": "Claude Desktop", gemini: "Gemini" };
const ADDRESS_RE = /^(localhost|::1|\[?[0-9a-f:]+\]?|(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3})$/i;

function uptimeLabel(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m ${seconds % 60}s`;
}

/** Validates failover parameters against CC Switch's ranges; returns invalid field labels. */
function invalidFailoverFields(config: AppProxyConfig, t: (key: MessageKey) => string) {
  const checks: Array<[boolean, MessageKey]> = [
    [config.maxRetries >= 0 && config.maxRetries <= 10, "settings.failover.maxRetries"],
    [config.streamingFirstByteTimeout >= 1 && config.streamingFirstByteTimeout <= 120, "settings.failover.streamingFirstByte"],
    [config.streamingIdleTimeout === 0 || (config.streamingIdleTimeout >= 60 && config.streamingIdleTimeout <= 600), "settings.failover.streamingIdle"],
    [config.nonStreamingTimeout >= 60 && config.nonStreamingTimeout <= 1200, "settings.failover.nonStreaming"],
    [config.circuitFailureThreshold >= 1 && config.circuitFailureThreshold <= 20, "settings.failover.failureThreshold"],
    [config.circuitSuccessThreshold >= 1 && config.circuitSuccessThreshold <= 10, "settings.failover.successThreshold"],
    [config.circuitTimeoutSeconds >= 0 && config.circuitTimeoutSeconds <= 300, "settings.failover.recoveryWait"],
    [config.circuitErrorRateThreshold >= 0 && config.circuitErrorRateThreshold <= 1, "settings.failover.errorRate"],
    [config.circuitMinRequests >= 5 && config.circuitMinRequests <= 100, "settings.failover.minRequests"],
  ];
  return checks.filter(([ok]) => !ok).map(([, key]) => t(key));
}

export function RoutingTab({
  routing,
  busy,
  providers,
  notify,
  run,
  saveConfig,
}: {
  routing: RoutingResult | null;
  busy: boolean;
  providers: ProviderOption[];
  notify: Notify;
  run: (command: string, args?: Record<string, unknown>) => Promise<RoutingResult | null>;
  saveConfig: (config: RoutingConfig) => Promise<RoutingResult | null>;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<RoutingConfig | null>(routing?.config ?? null);
  const [failoverApp, setFailoverApp] = useState<ProxyAppId>("codex");
  const [queuePick, setQueuePick] = useState("");
  const [proxyPassword, setProxyPassword] = useState("");
  const [proxyCheck, setProxyCheck] = useState("");

  useEffect(() => {
    if (routing?.config) setDraft(routing.config);
  }, [routing?.config]);

  const status = routing?.runtime;
  const running = Boolean(status?.running);
  const appConfig = draft?.apps.find((item) => item.appId === failoverApp);
  const queue = routing?.queues[failoverApp] ?? [];
  const queueCandidates = useMemo(
    () => providers.filter((item) => item.appId === failoverApp && !queue.some((entry) => entry.providerId === item.id)),
    [failoverApp, providers, queue],
  );

  if (!draft) {
    return <div className="st-tab-body"><p className="st-muted">{t("common.loading")}</p></div>;
  }

  const patchDraft = (patch: Partial<RoutingConfig>) => setDraft({ ...draft, ...patch });
  const patchApp = (patch: Partial<AppProxyConfig>) =>
    setDraft({ ...draft, apps: draft.apps.map((item) => (item.appId === failoverApp ? { ...item, ...patch } : item)) });
  // Immediate-effect toggles persist the whole routing config at once.
  const commit = async (next: RoutingConfig) => {
    setDraft(next);
    const result = await saveConfig(next);
    if (!result && routing?.config) setDraft(routing.config);
  };

  const addressValid = ADDRESS_RE.test(draft.listenAddress.trim());
  const portValid = Number.isInteger(draft.listenPort) && draft.listenPort >= 1024 && draft.listenPort <= 65535;

  const saveBasic = async () => {
    if (!addressValid) return notify("error", t("settings.routing.invalidAddress"));
    if (!portValid) return notify("error", t("settings.routing.invalidPort"));
    const result = await saveConfig(draft);
    if (result) notify("ok", running ? t("settings.routing.restartRequired") : t("settings.saved"));
  };

  const saveFailover = async () => {
    if (!appConfig) return;
    const invalid = invalidFailoverFields(appConfig, t);
    if (invalid.length) return notify("error", t("settings.failover.outOfRange", { fields: invalid.join("、") }));
    const result = await saveConfig(draft);
    if (result) notify("ok", t("settings.saved"));
  };

  const toggleRouting = async (value: boolean) => {
    if (value && !window.confirm(t("settings.routing.enableConfirm"))) return;
    await run(SETTINGS_COMMANDS.setRoutingEnabled, { request: { enabled: value } });
  };

  const testProxy = async () => {
    setProxyCheck("");
    try {
      const result = await invokeCommand<ProxyTestResult>(SETTINGS_COMMANDS.testGlobalProxy, {
        request: { url: draft.globalProxyUrl, username: draft.globalProxyUsername, password: proxyPassword || undefined },
      });
      setProxyCheck(result?.ok ? t("settings.globalProxy.testSuccess", { latency: result.latencyMs ?? 0 }) : t("settings.globalProxy.testFailed", { error: result?.message ?? "" }));
    } catch (error) {
      setProxyCheck(t("settings.globalProxy.testFailed", { error: error instanceof Error ? error.message : "" }));
    }
  };

  const scanProxy = async () => {
    try {
      const result = await invokeCommand<ProxyScanResult>(SETTINGS_COMMANDS.scanLocalProxies);
      const first = result?.candidates?.[0];
      if (first) patchDraft({ globalProxyUrl: first });
      setProxyCheck(first ? result.candidates.join("  ") : t("settings.globalProxy.scanNone"));
    } catch {
      setProxyCheck(t("settings.globalProxy.scanNone"));
    }
  };

  const saveProxy = async () => {
    const result = await saveConfig({ ...draft, globalProxyPassword: proxyPassword || undefined });
    if (result) {
      setProxyPassword("");
      notify("ok", t("settings.saved"));
    }
  };

  return (
    <div className="st-tab-body">
      <SettingsSection
        badge={<span className={`st-badge${running ? " on" : ""}`}><Activity aria-hidden="true" />{running ? t("settings.routing.running") : t("settings.routing.stopped")}</span>}
        defaultOpen
        description={t("settings.routing.local.description")}
        icon={Server}
        title={t("settings.routing.local.title")}
        tone="var(--workspace-success)"
      >
        <ToggleRow checked={draft.enabled} disabled={busy} onChange={(value) => void toggleRouting(value)} title={t("settings.routing.enable")} />
        <ToggleRow checked={draft.showRoutingToggleOnMain} description={t("settings.routing.showToggleOnMainDescription")} onChange={(value) => void commit({ ...draft, showRoutingToggleOnMain: value })} title={t("settings.routing.showToggleOnMain")} />
        {running && status ? (
          <div className="st-stats">
            <div><small>{t("settings.routing.stats.activeConnections")}</small><strong>{status.activeConnections}</strong></div>
            <div><small>{t("settings.routing.stats.totalRequests")}</small><strong>{status.totalRequests}</strong></div>
            <div><small>{t("settings.routing.stats.successRate")}</small><strong>{Math.round(status.successRate * 100)}%</strong></div>
            <div><small>{t("settings.routing.stats.uptime")}</small><strong>{uptimeLabel(status.uptimeSeconds)}</strong></div>
          </div>
        ) : null}
        <p className="st-muted">{t("settings.routing.takeoverHint")}</p>
        <div className="st-takeover">
          {draft.apps.map((app) => (
            <label key={app.appId}>
              <span>{APP_LABELS[app.appId]}</span>
              <Switch checked={app.takeover} disabled={busy} label={APP_LABELS[app.appId]} onChange={(value) => void run(SETTINGS_COMMANDS.setAppTakeover, { request: { appId: app.appId, takeover: value } })} />
            </label>
          ))}
        </div>
        <div className="st-grid-2">
          <label className={`st-input${addressValid ? "" : " invalid"}`}>
            <span>{t("settings.routing.listenAddress")}</span>
            <input onChange={(event) => patchDraft({ listenAddress: event.target.value })} value={draft.listenAddress} />
            <small>{t("settings.routing.listenAddressDescription")}</small>
          </label>
          <NumberField hint={t("settings.routing.listenPortDescription")} label={t("settings.routing.listenPort")} max={65535} min={1024} onChange={(value) => patchDraft({ listenPort: value })} value={draft.listenPort} />
        </div>
        <ToggleRow checked={draft.enableLogging} description={t("settings.routing.enableLoggingDescription")} onChange={(value) => patchDraft({ enableLogging: value })} title={t("settings.routing.enableLogging")} />
        <div className="st-actions"><button className="st-button primary" disabled={busy} onClick={() => void saveBasic()} type="button">{t("settings.routing.saveConfig")}</button></div>
      </SettingsSection>

      <SettingsSection description={t("settings.failover.description")} icon={Activity} title={t("settings.failover.title")} tone="var(--workspace-warning)">
        <ToggleRow checked={draft.showFailoverToggleOnMain} description={t("settings.failover.showToggleOnMainDescription")} onChange={(value) => void commit({ ...draft, showFailoverToggleOnMain: value })} title={t("settings.failover.showToggleOnMain")} />
        {!running ? <p className="st-warning">{t("settings.failover.proxyRequired")}</p> : null}
        <div className="st-app-tabs" role="tablist">
          {draft.apps.map((app) => (
            <button aria-selected={failoverApp === app.appId} className={failoverApp === app.appId ? "on" : ""} key={app.appId} onClick={() => setFailoverApp(app.appId)} role="tab" type="button">{APP_LABELS[app.appId]}</button>
          ))}
        </div>
        {appConfig ? (
          <fieldset className="st-fieldset" disabled={!running || busy}>
            <ToggleRow checked={appConfig.autoFailoverEnabled} description={t("settings.failover.autoSwitchDescription")} onChange={(value) => patchApp({ autoFailoverEnabled: value })} title={t("settings.failover.autoSwitch")} />
            <h4>{t("settings.failover.queue")}</h4>
            {queue.length ? (
              <ol className="st-queue">
                {queue.map((entry) => (
                  <li key={entry.providerId}>
                    <span className="st-queue-priority">P{entry.priority}</span>
                    <strong>{entry.name}</strong>
                    <span className={`st-circuit ${entry.circuitState}`}>{t(`settings.failover.circuit.${entry.circuitState}` as MessageKey)}</span>
                    {entry.circuitState !== "closed" ? <button className="st-icon-button" onClick={() => void run(SETTINGS_COMMANDS.resetCircuitBreaker, { request: { appId: failoverApp, providerId: entry.providerId } })} title={t("settings.failover.resetCircuit")} type="button"><RotateCcw aria-hidden="true" /></button> : null}
                    <button className="st-icon-button" onClick={() => void run(SETTINGS_COMMANDS.removeFailoverQueue, { request: { appId: failoverApp, providerId: entry.providerId } })} title={t("settings.failover.remove")} type="button"><Trash2 aria-hidden="true" /></button>
                  </li>
                ))}
              </ol>
            ) : <p className="st-muted">{t("settings.failover.queueEmpty")}</p>}
            <div className="st-inline">
              <select aria-label={t("settings.failover.selectProvider")} className="st-select" onChange={(event) => setQueuePick(event.target.value)} value={queuePick}>
                <option value="">{t("settings.failover.selectProvider")}</option>
                {queueCandidates.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
              </select>
              <button className="st-button" disabled={!queuePick} onClick={() => { void run(SETTINGS_COMMANDS.addFailoverQueue, { request: { appId: failoverApp, providerId: queuePick } }); setQueuePick(""); }} type="button"><Plus aria-hidden="true" />{t("settings.failover.add")}</button>
            </div>
            <h4>{t("settings.failover.retrySettings")}</h4>
            <div className="st-grid-2">
              <NumberField hint={t("settings.failover.maxRetriesHint")} label={t("settings.failover.maxRetries")} max={10} min={0} onChange={(value) => patchApp({ maxRetries: value })} value={appConfig.maxRetries} />
              <NumberField hint={t("settings.failover.streamingFirstByteHint")} label={t("settings.failover.streamingFirstByte")} max={120} min={1} onChange={(value) => patchApp({ streamingFirstByteTimeout: value })} value={appConfig.streamingFirstByteTimeout} />
              <NumberField hint={t("settings.failover.streamingIdleHint")} label={t("settings.failover.streamingIdle")} max={600} min={0} onChange={(value) => patchApp({ streamingIdleTimeout: value })} value={appConfig.streamingIdleTimeout} />
              <NumberField hint={t("settings.failover.nonStreamingHint")} label={t("settings.failover.nonStreaming")} max={1200} min={60} onChange={(value) => patchApp({ nonStreamingTimeout: value })} value={appConfig.nonStreamingTimeout} />
            </div>
            <h4>{t("settings.failover.circuitSettings")}</h4>
            <div className="st-grid-2">
              <NumberField hint={t("settings.failover.failureThresholdHint")} label={t("settings.failover.failureThreshold")} max={20} min={1} onChange={(value) => patchApp({ circuitFailureThreshold: value })} value={appConfig.circuitFailureThreshold} />
              <NumberField hint={t("settings.failover.recoveryWaitHint")} label={t("settings.failover.recoveryWait")} max={300} min={0} onChange={(value) => patchApp({ circuitTimeoutSeconds: value })} value={appConfig.circuitTimeoutSeconds} />
              <NumberField hint={t("settings.failover.successThresholdHint")} label={t("settings.failover.successThreshold")} max={10} min={1} onChange={(value) => patchApp({ circuitSuccessThreshold: value })} value={appConfig.circuitSuccessThreshold} />
              <NumberField hint={t("settings.failover.errorRateHint")} label={t("settings.failover.errorRate")} max={100} min={0} onChange={(value) => patchApp({ circuitErrorRateThreshold: value / 100 })} value={Math.round(appConfig.circuitErrorRateThreshold * 100)} />
              <NumberField hint={t("settings.failover.minRequestsHint")} label={t("settings.failover.minRequests")} max={100} min={5} onChange={(value) => patchApp({ circuitMinRequests: value })} value={appConfig.circuitMinRequests} />
            </div>
            <div className="st-actions"><button className="st-button primary" onClick={() => void saveFailover()} type="button">{t("settings.routing.saveConfig")}</button></div>
          </fieldset>
        ) : null}
      </SettingsSection>

      <SettingsSection description={t("settings.rectifier.description")} icon={Zap} title={t("settings.rectifier.title")} tone="oklch(0.68 0.16 300)">
        <ToggleRow checked={draft.rectifier.enabled} description={t("settings.rectifier.enabledDescription")} onChange={(value) => void commit({ ...draft, rectifier: { ...draft.rectifier, enabled: value } })} title={t("settings.rectifier.enabled")} />
        {(["thinkingSignature", "thinkingBudget", "mediaFallback", "mediaHeuristic"] as const).map((key) => (
          <ToggleRow
            checked={draft.rectifier[key]}
            description={t(`settings.rectifier.${key}Description` as MessageKey)}
            disabled={!draft.rectifier.enabled}
            key={key}
            onChange={(value) => void commit({ ...draft, rectifier: { ...draft.rectifier, [key]: value } })}
            title={t(`settings.rectifier.${key}` as MessageKey)}
          />
        ))}
      </SettingsSection>

      <SettingsSection description={t("settings.globalProxy.description")} icon={Globe2} title={t("settings.globalProxy.title")} tone="oklch(0.72 0.12 200)">
        <label className="st-input">
          <span>{t("settings.globalProxy.label")}</span>
          <input onChange={(event) => patchDraft({ globalProxyUrl: event.target.value })} placeholder="http://127.0.0.1:7890" value={draft.globalProxyUrl} />
          <small>{t("settings.globalProxy.hint")}</small>
        </label>
        <div className="st-grid-2">
          <label className="st-input"><span>{t("settings.globalProxy.username")}</span><input autoComplete="off" onChange={(event) => patchDraft({ globalProxyUsername: event.target.value })} value={draft.globalProxyUsername} /></label>
          <label className="st-input"><span>{t("settings.globalProxy.password")}</span><input autoComplete="new-password" onChange={(event) => setProxyPassword(event.target.value)} placeholder={t("settings.globalProxy.passwordKept")} type="password" value={proxyPassword} /></label>
        </div>
        {proxyCheck ? <p className="st-muted" role="status">{proxyCheck}</p> : null}
        <div className="st-actions">
          <button className="st-button" onClick={() => void testProxy()} type="button">{t("settings.globalProxy.test")}</button>
          <button className="st-button" onClick={() => void scanProxy()} type="button">{t("settings.globalProxy.scan")}</button>
          <button className="st-button" onClick={() => { patchDraft({ globalProxyUrl: "", globalProxyUsername: "" }); setProxyPassword(""); }} type="button">{t("settings.globalProxy.clear")}</button>
          <button className="st-button primary" disabled={busy} onClick={() => void saveProxy()} type="button">{t("settings.save")}</button>
        </div>
      </SettingsSection>
    </div>
  );
}
