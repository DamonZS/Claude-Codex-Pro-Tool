import { useCallback, useEffect, useMemo, useState } from "react";
import { CheckCircle2, ClipboardCopy, FileCode2, Pencil, Plus, RefreshCw, Trash2, Upload, X } from "lucide-react";

import { invokeCommand } from "@/tauriBridge";
import {
  AGENT_PROVIDER_COMMANDS,
  agentProviderApp,
  emptyAgentProvider,
  type AgentApiFormat,
  type AgentApplyResult,
  type AgentProvider,
  type AgentProviderAppId,
  type AgentProvidersResult,
} from "./agentProviderContract";
import "../settings/settings.css";
import "./agent-provider.css";

const FORMAT_LABELS: Record<AgentApiFormat, string> = {
  "openai-chat": "OpenAI Chat Completions",
  "openai-responses": "OpenAI Responses",
  anthropic: "Anthropic Messages",
};

const MODE_HINTS = {
  switch: "同一时间只启用一个供应商，切换时只改写关键字段，其余配置保持不变。",
  additive: "多个供应商可同时写入该 Agent 的配置文件，在 Agent 内自行选择使用。",
  manual: "该应用的密钥保存在加密存储中，CCP 不直接改写。点击“复制配置”后粘贴到应用设置里。",
} as const;

type Toast = { id: number; tone: "ok" | "error"; text: string };

function isHttpUrl(value: string) {
  try {
    const url = new URL(value.trim());
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

/** Field errors for the editor; empty object means the draft can be saved. */
function validateDraft(draft: AgentProvider, mode: "switch" | "additive" | "manual") {
  const errors: Partial<Record<"name" | "baseUrl" | "apiKey" | "models" | "defaultModel", string>> = {};
  if (!draft.name.trim()) errors.name = "请填写供应商名称";
  if (!isHttpUrl(draft.baseUrl)) errors.baseUrl = "请填写以 http:// 或 https:// 开头的地址";
  if (!draft.hasApiKey && !draft.apiKey?.trim()) errors.apiKey = "请填写 API Key";
  if (!draft.models.length) errors.models = "至少填写一个模型";
  if (mode === "switch" && draft.defaultModel && !draft.models.includes(draft.defaultModel)) {
    errors.defaultModel = "默认模型必须在模型列表中";
  }
  return errors;
}

export function AgentProviderPanel({ appId }: { appId: AgentProviderAppId }) {
  const app = agentProviderApp(appId);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const notify = useCallback((tone: Toast["tone"], text: string) => {
    const id = Date.now() + Math.random();
    setToasts((current) => [...current.slice(-2), { id, tone, text }]);
    window.setTimeout(() => setToasts((current) => current.filter((toast) => toast.id !== id)), tone === "error" ? 6000 : 3500);
  }, []);
  const [data, setData] = useState<AgentProvidersResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [draft, setDraft] = useState<AgentProvider | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invokeCommand<AgentProvidersResult>(AGENT_PROVIDER_COMMANDS.list);
      setData(result);
      if (result.status !== "ok") notify("error", result.message);
    } catch (error) {
      notify("error", error instanceof Error ? error.message : "读取供应商失败");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const providers = useMemo(
    () => (data?.providers ?? []).filter((item) => item.appId === appId).sort((a, b) => a.sortIndex - b.sortIndex),
    [appId, data],
  );
  const state = data?.states?.[appId];

  /** Runs a mutating command; the backend always answers with the full list. */
  const mutate = async <T extends AgentProvidersResult>(id: string, command: string, request: Record<string, unknown>, okText: string) => {
    setBusyId(id);
    try {
      const result = await invokeCommand<T>(command, { request });
      if (result.providers) setData(result);
      if (result.status !== "ok") {
        notify("error", result.message);
        return null;
      }
      notify("ok", okText);
      return result;
    } catch (error) {
      notify("error", error instanceof Error ? error.message : "操作失败");
      return null;
    } finally {
      setBusyId(null);
    }
  };

  const apply = async (provider: AgentProvider) => {
    const result = await mutate<AgentApplyResult>(provider.id, AGENT_PROVIDER_COMMANDS.apply, { appId, id: provider.id }, app.mode === "switch" ? `已切换到 ${provider.name}` : `已写入 ${app.label}`);
    if (app.mode === "manual" && result?.reveal) {
      const { baseUrl, apiKey, model } = result.reveal;
      try {
        await navigator.clipboard.writeText(`Base URL: ${baseUrl}\nAPI Key: ${apiKey}\nModel: ${model}`);
        notify("ok", "已复制 Base URL、API Key 与模型，请在 Cursor 设置 → Models 中开启 Override OpenAI Base URL 后粘贴");
      } catch {
        notify("error", "复制到剪贴板失败");
      }
    }
  };

  const remove = async (provider: AgentProvider) => {
    if (!window.confirm(`删除供应商“${provider.name}”？如已写入 ${app.label}，会同时从其配置中移除。`)) return;
    await mutate(provider.id, AGENT_PROVIDER_COMMANDS.delete, { appId, id: provider.id }, "供应商已删除");
  };

  const save = async (next: AgentProvider) => {
    const request = { provider: { ...next, apiKey: next.apiKey?.trim() || undefined } };
    const result = await mutate(next.id || "new", AGENT_PROVIDER_COMMANDS.save, request, "供应商已保存");
    if (result) setDraft(null);
  };

  return (
    <div className="st-root ap-root">
      <div className="ap-head">
        <div className="ap-head-copy">
          <strong>{app.label} 供应商</strong>
          <small>{MODE_HINTS[app.mode]}</small>
        </div>
        <div className="st-actions">
          <button className="st-icon-button" disabled={loading} onClick={() => void load()} title="刷新" type="button">
            <RefreshCw aria-hidden="true" className={loading ? "spin" : ""} />
          </button>
          <button className="st-button primary" onClick={() => setDraft(emptyAgentProvider(appId))} type="button">
            <Plus aria-hidden="true" />添加供应商
          </button>
        </div>
      </div>

      <p className="ap-path">
        <FileCode2 aria-hidden="true" />
        <span>{state?.configPath || app.configHint}</span>
        {state && !state.installed ? <em>未检测到 {app.label}</em> : null}
      </p>

      {loading && !data ? <p className="st-muted">正在读取供应商…</p> : null}
      {!loading && !providers.length ? (
        <div className="ap-empty">
          <strong>还没有 {app.label} 供应商</strong>
          <small>添加一个兼容 {app.formats.map((format) => FORMAT_LABELS[format]).join(" / ")} 的接口即可。</small>
        </div>
      ) : null}

      <ul className="ap-list">
        {providers.map((provider) => {
          const active = app.mode === "switch" && state?.activeId === provider.id;
          const applied = app.mode === "additive" && !!state?.appliedIds.includes(provider.id);
          const busy = busyId === provider.id;
          return (
            <li className={`ap-card${active || applied ? " on" : ""}`} key={provider.id}>
              <span aria-hidden="true" className="ap-avatar">{(provider.name || "P").slice(0, 1).toUpperCase()}</span>
              <span className="ap-card-main">
                <span className="ap-title">
                  <strong>{provider.name}</strong>
                  {active ? <span className="st-badge on"><CheckCircle2 aria-hidden="true" />使用中</span> : null}
                  {applied ? <span className="st-badge on"><CheckCircle2 aria-hidden="true" />已写入</span> : null}
                </span>
                <small className="ap-url">{provider.baseUrl}</small>
                <small className="ap-meta">
                  {FORMAT_LABELS[provider.apiFormat]} · {provider.models.length} 个模型
                  {provider.defaultModel ? ` · 默认 ${provider.defaultModel}` : ""}
                </small>
              </span>
              <span className="ap-card-actions">
                {app.mode === "manual" ? (
                  <button className="st-button" disabled={busy} onClick={() => void apply(provider)} type="button"><ClipboardCopy aria-hidden="true" />复制配置</button>
                ) : app.mode === "additive" && applied ? (
                  <button className="st-button" disabled={busy} onClick={() => void mutate(provider.id, AGENT_PROVIDER_COMMANDS.unapply, { appId, id: provider.id }, `已从 ${app.label} 移除`)} type="button"><X aria-hidden="true" />移除</button>
                ) : (
                  <button className="st-button primary" disabled={busy || active} onClick={() => void apply(provider)} type="button">
                    <Upload aria-hidden="true" />{app.mode === "switch" ? (active ? "已启用" : "启用") : "写入"}
                  </button>
                )}
                <button aria-label={`编辑 ${provider.name}`} className="st-icon-button" disabled={busy} onClick={() => setDraft({ ...provider, apiKey: "" })} type="button"><Pencil aria-hidden="true" /></button>
                <button aria-label={`删除 ${provider.name}`} className="st-icon-button ap-danger" disabled={busy} onClick={() => void remove(provider)} type="button"><Trash2 aria-hidden="true" /></button>
              </span>
            </li>
          );
        })}
      </ul>

      {toasts.length ? (
        <div aria-live="polite" className="st-toasts">
          {toasts.map((toast) => (
            <div className={`st-toast${toast.tone === "error" ? " error" : ""}`} key={toast.id} role={toast.tone === "error" ? "alert" : "status"}>
              <span>{toast.text}</span>
              <button aria-label="关闭提示" onClick={() => setToasts((current) => current.filter((item) => item.id !== toast.id))} type="button"><X aria-hidden="true" /></button>
            </div>
          ))}
        </div>
      ) : null}

      {draft ? <AgentProviderEditor busy={busyId !== null} draft={draft} mode={app.mode} formats={app.formats} label={app.label} onCancel={() => setDraft(null)} onSave={(next) => void save(next)} /> : null}
    </div>
  );
}

function AgentProviderEditor({
  draft: initial,
  mode,
  formats,
  label,
  busy,
  onCancel,
  onSave,
}: {
  draft: AgentProvider;
  mode: "switch" | "additive" | "manual";
  formats: AgentApiFormat[];
  label: string;
  busy: boolean;
  onCancel: () => void;
  onSave: (draft: AgentProvider) => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [modelsText, setModelsText] = useState(initial.models.join("\n"));
  const [touched, setTouched] = useState(false);
  const patch = (next: Partial<AgentProvider>) => setDraft((current) => ({ ...current, ...next }));

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  const models = useMemo(() => [...new Set(modelsText.split(/[\n,]/).map((item) => item.trim()).filter(Boolean))], [modelsText]);
  const candidate = { ...draft, models, defaultModel: draft.defaultModel || (mode === "switch" ? models[0] ?? "" : "") };
  const errors = validateDraft(candidate, mode);
  const show = (key: keyof typeof errors) => (touched ? errors[key] : undefined);

  const submit = () => {
    setTouched(true);
    if (Object.keys(errors).length) return;
    onSave(candidate);
  };

  return (
    <div className="ap-editor-backdrop" onClick={onCancel} role="presentation">
      <section aria-label={`${initial.id ? "编辑" : "添加"} ${label} 供应商`} aria-modal="true" className="ap-editor" onClick={(event) => event.stopPropagation()} role="dialog">
        <header className="ap-editor-head">
          <strong>{initial.id ? "编辑" : "添加"} {label} 供应商</strong>
          <button aria-label="关闭" className="st-icon-button" onClick={onCancel} type="button"><X aria-hidden="true" /></button>
        </header>
        <div className="ap-editor-body">
          <label className={`st-input${show("name") ? " invalid" : ""}`}>
            <span>名称</span>
            <input autoFocus onChange={(event) => patch({ name: event.target.value })} placeholder="例如：DeepSeek 官方" value={draft.name} />
            {show("name") ? <small className="ap-error">{show("name")}</small> : null}
          </label>
          <label className={`st-input${show("baseUrl") ? " invalid" : ""}`}>
            <span>Base URL</span>
            <input inputMode="url" onChange={(event) => patch({ baseUrl: event.target.value })} placeholder="https://api.example.com/v1" spellCheck={false} value={draft.baseUrl} />
            {show("baseUrl") ? <small className="ap-error">{show("baseUrl")}</small> : <small>填到 /v1 这一级，CCP 会按接口格式补全请求路径。</small>}
          </label>
          <label className={`st-input${show("apiKey") ? " invalid" : ""}`}>
            <span>API Key</span>
            <input autoComplete="off" onChange={(event) => patch({ apiKey: event.target.value })} placeholder={draft.hasApiKey ? "已保存，留空则不修改" : "sk-..."} spellCheck={false} type="password" value={draft.apiKey ?? ""} />
            {show("apiKey") ? <small className="ap-error">{show("apiKey")}</small> : null}
          </label>
          {formats.length > 1 ? (
            <label className="st-input">
              <span>接口格式</span>
              <select className="st-select" onChange={(event) => patch({ apiFormat: event.target.value as AgentApiFormat })} value={draft.apiFormat}>
                {formats.map((format) => <option key={format} value={format}>{FORMAT_LABELS[format]}</option>)}
              </select>
            </label>
          ) : null}
          <label className={`st-input${show("models") ? " invalid" : ""}`}>
            <span>模型</span>
            <textarea onChange={(event) => setModelsText(event.target.value)} placeholder={"每行一个，或用逗号分隔"} rows={4} spellCheck={false} value={modelsText} />
            {show("models") ? <small className="ap-error">{show("models")}</small> : null}
          </label>
          {mode === "switch" && models.length ? (
            <label className={`st-input${show("defaultModel") ? " invalid" : ""}`}>
              <span>默认模型</span>
              <select className="st-select" onChange={(event) => patch({ defaultModel: event.target.value })} value={candidate.defaultModel}>
                {models.map((model) => <option key={model} value={model}>{model}</option>)}
              </select>
              {show("defaultModel") ? <small className="ap-error">{show("defaultModel")}</small> : null}
            </label>
          ) : null}
          <label className="st-input">
            <span>备注</span>
            <input onChange={(event) => patch({ notes: event.target.value })} value={draft.notes} />
          </label>
        </div>
        <footer className="st-actions ap-editor-foot">
          <button className="st-button" onClick={onCancel} type="button">取消</button>
          <button className="st-button primary" disabled={busy} onClick={submit} type="button">保存</button>
        </footer>
      </section>
    </div>
  );
}
