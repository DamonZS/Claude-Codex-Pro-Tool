import { useEffect, useMemo, useState } from "react";
import { Check, FileText, Github, LoaderCircle, Pencil, Play, Plus, RefreshCw, Terminal, TriangleAlert, Trash2, Upload, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type {
  ClientDeployResult,
  LogsResult,
  PromptLibraryResult,
  PromptCompositionSource,
  SaveSystemPromptRequest,
  SystemPromptItem,
  SystemPromptMode,
  SystemPromptResult,
} from "@/types";

type Props = {
  actions: AppActions;
  clientDeploy: ClientDeployResult | null;
  library: PromptLibraryResult | null;
  prompts: SystemPromptResult | null;
  logs: LogsResult | null;
};

function promptOperationLines(logs: LogsResult | null) {
  const text = logs?.text ?? "";
  return text.split(/\r?\n/).filter(Boolean).flatMap((line) => {
    try {
      const record = JSON.parse(line) as {
        timestamp_ms?: number;
        event?: string;
        detail?: Record<string, unknown>;
      };
      if (record.event !== "manager.prompt_shell.step" && record.event !== "manager.leila.shell_output") return [];
      const detail = record.detail ?? {};
      const time = typeof record.timestamp_ms === "number"
        ? new Date(record.timestamp_ms).toLocaleTimeString()
        : "";
      const content = record.event === "manager.leila.shell_output"
        ? String(detail.line ?? "")
        : [detail.step, detail.result].filter((value) => typeof value === "string").join(" · ");
      return [{
        time,
        area: String(detail.area ?? "操作"),
        target: String(detail.target ?? ""),
        path: typeof detail.path === "string" ? detail.path : "",
        content,
        failed: String(detail.result ?? detail.level ?? "").includes("失败") || detail.level === "error",
        succeeded: /成功|完成|已部署|已安装|通过|success|completed|installed|passed/i.test(content),
      }];
    } catch {
      return /提示词|客户端|破甲|技能|工具|环境/.test(line)
        ? [{ time: "", area: "日志", target: "", path: "", content: line, failed: false, succeeded: false }]
        : [];
    }
  });
}

const EMPTY_FORM: SaveSystemPromptRequest = {
  id: "", title: "", filename: "", description: "", category: "软件开发", content: "",
};

function PromptCard({ actions, item, active, mode, onEdit }: {
  actions: AppActions;
  item: SystemPromptItem;
  active: boolean;
  mode: SystemPromptMode;
  onEdit: (item: SystemPromptItem) => void;
}) {
  return (
    <article className={`system-prompt-card${active ? " is-current" : ""}`}>
      <header>
        <span className="system-prompt-file-icon"><FileText aria-hidden="true" /></span>
        <strong title={item.title}>{item.title}</strong>
        <label className="system-prompt-switch" title={active ? "停用" : "启用"}>
          <input
            type="checkbox"
            checked={active}
            onChange={() => void (active ? actions.disableSystemPrompt() : actions.enableSystemPrompt(item.id, mode))}
          />
          <span aria-hidden="true" />
        </label>
      </header>
      <p>{item.description || "自定义 Codex 系统提示词"}</p>
      <footer>
        <span className="system-prompt-category">{item.category}</span>
        {active ? <span className="system-prompt-current"><Check aria-hidden="true" />当前</span> : null}
        {!item.builtin ? (
          <span className="system-prompt-card-actions">
            <button type="button" title="编辑" onClick={() => onEdit(item)}><Pencil aria-hidden="true" /></button>
            <button type="button" title="删除" onClick={() => void actions.deleteSystemPrompt(item.id)}><Trash2 aria-hidden="true" /></button>
          </span>
        ) : null}
      </footer>
    </article>
  );
}

export function SystemPromptScreen({ actions, clientDeploy, library, logs, prompts }: Props) {
  const [selectedTargetId, setSelectedTargetId] = useState("");
  const [selectedVersionId, setSelectedVersionId] = useState("");
  const [deployBusy, setDeployBusy] = useState(false);
  const [category, setCategory] = useState("全部");
  const [form, setForm] = useState<SaveSystemPromptRequest | null>(null);
  const [syncOpen, setSyncOpen] = useState(false);
  const [syncUrl, setSyncUrl] = useState("");
  const items = prompts?.prompts ?? [];
  const categories = useMemo(() => ["全部", ...Array.from(new Set(items.map((item) => item.category))).sort((a, b) => a.localeCompare(b, "zh-CN"))], [items]);
  const filtered = category === "全部" ? items : items.filter((item) => item.category === category);
  const failed = prompts !== null && !statusOk(prompts.status);
  const activeMode = prompts?.mode;
  const deployTargets = useMemo(
    () => [...(clientDeploy?.targets ?? [])].sort((a, b) => Number(b.installed) - Number(a.installed)),
    [clientDeploy],
  );
  const deployVersions = useMemo(
    () => (library?.prompts ?? []).filter((entry) => !selectedTargetId || !entry.targets.length || entry.targets.includes(selectedTargetId)),
    [library, selectedTargetId],
  );
  const selectedTarget = deployTargets.find((target) => target.targetId === selectedTargetId) ?? null;
  const selectedVersion = deployVersions.find((entry) => entry.id === selectedVersionId) ?? null;
  const libraryOffline = library !== null && library.online === false;
  const operationLines = promptOperationLines(logs);

  useEffect(() => {
    if (!selectedTargetId || !deployTargets.some((target) => target.targetId === selectedTargetId)) {
      setSelectedTargetId(deployTargets.find((target) => target.deployable)?.targetId ?? "");
    }
  }, [deployTargets, selectedTargetId]);

  useEffect(() => {
    if (!selectedVersionId || !deployVersions.some((entry) => entry.id === selectedVersionId)) {
      setSelectedVersionId(deployVersions[0]?.id ?? "");
    }
  }, [deployVersions, selectedVersionId]);

  const edit = (item: SystemPromptItem) => setForm({
    id: item.id, title: item.title, filename: item.filename, description: item.description, category: item.category, content: item.content,
  });
  const deployBundle = async () => {
    if (!selectedTarget || !selectedTarget.deployable || !selectedVersion) {
      return;
    }
    setDeployBusy(true);
    try {
      const content = await actions.fetchPromptContent(selectedVersion.id);
      if (!content || !statusOk(content.status)) {
        await actions.refreshLogs();
        return;
      }
      const promptResult = await actions.deployPromptToClients([{ targetId: selectedTarget.targetId, content: content.content }]);
      if (!promptResult) return;
      if (promptResult.results?.some((row) => row.ok)) {
        const skillNames = selectedVersion.skills;
        if (skillNames.length) {
          await actions.installSkillsToClients([selectedTarget.targetId], skillNames);
        }
        for (const tool of (library?.tools ?? []).filter((entry) => selectedVersion.tools.includes(entry.id))) {
          await actions.installToolPackage(tool.id);
        }
      }
      await actions.refreshLogs();
    } finally {
      setDeployBusy(false);
    }
  };

  const restoreTargets = async (targetIds: string[]) => {
    await actions.restoreClientDeploy(targetIds);
    await actions.refreshLogs();
  };


  const save = async () => {
    if (!form) return;
    const result = await actions.saveSystemPrompt(form);
    if (result && statusOk(result.status)) setForm(null);
  };

  return (
    <section className="system-prompt-screen" aria-labelledby="system-prompt-title">
      <header className="ops-page-heading system-prompt-heading">
        <div>
          <p className="system-prompt-eyebrow">PROMPT INJECTION</p>
          <h1 id="system-prompt-title">一键管理指令提示词</h1>
          <p>管理 Codex 的本地 Markdown 指令模板，修改配置前自动备份。</p>
        </div>
        <div className="system-prompt-toolbar">
          <Button variant="outline" onClick={() => setSyncOpen(true)}><Github aria-hidden="true" />同步 GitHub 模板</Button>
          <Button variant="outline" onClick={() => void actions.importSystemPrompt()}><Upload aria-hidden="true" />导入 md</Button>
          <Button onClick={() => setForm({ ...EMPTY_FORM })}><Plus aria-hidden="true" />添加提示词</Button>
        </div>
      </header>

      <section aria-label="统一部署控制台" className="prompt-deploy-panel prompt-bundle-console">
        <header className="prompt-deploy-header">
          <div>
            <strong>统一部署</strong>
            <small>选择一个已安装客户端和版本；部署会按顺序写入提示词、技能与工具，并保留可还原状态。</small>
          </div>
          <div className="prompt-deploy-actions">
            <Button disabled={deployBusy || !selectedTarget?.deployable || !selectedVersion} onClick={() => void deployBundle()}>
              {deployBusy ? <LoaderCircle className="spin" aria-hidden="true" /> : <Play aria-hidden="true" />}部署
            </Button>
            <Button disabled={!selectedTarget?.managed} onClick={() => void restoreTargets([selectedTargetId])} variant="outline">还原当前目标</Button>
            <Button onClick={() => void restoreTargets([])} variant="outline">一键全部还原</Button>
            <Button onClick={() => void actions.refreshPromptLibrary(false)} variant="outline"><RefreshCw aria-hidden="true" />刷新资源</Button>
          </div>
        </header>

        {!clientDeploy ? (
          <div className="prompt-deploy-empty" role="status"><LoaderCircle className="spin" aria-hidden="true" /><span>正在检测本机客户端...</span><Button onClick={() => void actions.refreshClientDeployTargets(false)} variant="outline">重新检测</Button></div>
        ) : !statusOk(clientDeploy.status) ? (
          <div className="prompt-deploy-warning" role="alert"><TriangleAlert aria-hidden="true" /><span>{clientDeploy.message || "客户端状态加载失败。"}</span><Button onClick={() => void actions.refreshClientDeployTargets(false)} variant="outline">重试</Button></div>
        ) : (
          <div className="prompt-bundle-selectors">
            <label>
              <span>部署目标</span>
              <select aria-label="选择部署目标" value={selectedTargetId} onChange={(event) => setSelectedTargetId(event.target.value)}>
                <option value="">选择客户端...</option>
                {deployTargets.map((target) => <option key={target.targetId} disabled={!target.deployable} value={target.targetId}>{target.displayName}{target.deployable ? (target.managed ? " · 已受管" : " · 已安装") : " · 未安装"}</option>)}
              </select>
              <small>{selectedTarget?.plannedPath ?? selectedTarget?.home ?? "未检测到数据目录"}</small>
            </label>
            <label>
              <span>部署版本</span>
              <select aria-label="选择部署版本" value={selectedVersionId} onChange={(event) => setSelectedVersionId(event.target.value)} title={selectedVersion?.description ?? ""}>
                <option value="">选择版本...</option>
                {deployVersions.map((entry) => <option key={entry.id} value={entry.id}>{entry.title}{entry.version ? ` · ${entry.version}` : ""}</option>)}
              </select>
              <small>{selectedVersion?.description || "版本简介将在这里显示。"}</small>
            </label>
          </div>
        )}

        {libraryOffline ? <div className="prompt-deploy-warning" role="alert"><TriangleAlert aria-hidden="true" /><span>内容库离线，无法读取版本正文和资源清单。</span></div> : null}

        <section className="prompt-shell-log-panel" aria-labelledby="prompt-runtime-status-title">
          <header className="prompt-shell-log-heading"><Terminal aria-hidden="true" /><strong id="prompt-runtime-status-title">运行状态</strong><span>{operationLines.length} 条</span></header>
          <div className="prompt-shell-log-content"><header><code>{logs?.path ?? "操作日志尚未加载"}</code><Button variant="outline" onClick={() => void actions.refreshLogs()}><RefreshCw aria-hidden="true" />刷新</Button></header>{operationLines.length ? <pre>{operationLines.map((line, index) => <span className={line.failed ? "is-failed" : line.succeeded ? "is-ok" : ""} key={`${line.time}-${index}`}>{line.time ? `[${line.time}] ` : ""}[{line.area}{line.target ? ` / ${line.target}` : ""}] {line.content}{line.path ? ` | ${line.path}` : ""}{"\n"}</span>)}</pre> : <p>{logs ? "暂无部署步骤记录。" : "正在等待操作日志。"}</p>}</div>
        </section>
      </section>

      <div className="system-prompt-filter-row">
        <div className="system-prompt-categories" role="tablist" aria-label="提示词分类">
          {categories.map((name) => <button key={name} type="button" role="tab" aria-selected={category === name} className={category === name ? "is-selected" : ""} onClick={() => setCategory(name)}>{name}</button>)}
        </div>
        <Button variant="ghost" onClick={() => void actions.refreshSystemPrompts(false)}><RefreshCw aria-hidden="true" />刷新</Button>
      </div>

      {prompts?.storageRecovered ? (
        <div className="system-prompt-storage-warning" role="status">
          <TriangleAlert aria-hidden="true" />
          <span>原状态文件暂时无法读取（权限不足或文件占用），已切换到当前用户可写的恢复存储；原文件未被覆盖，其中的旧自定义提示词暂不可见。</span>
        </div>
      ) : null}

      {prompts === null ? (
        <div className="system-prompt-empty"><LoaderCircle className="spin" /><strong>正在加载系统提示词</strong></div>
      ) : failed ? (
        <div className="system-prompt-empty is-error"><strong>系统提示词加载失败</strong><p>{prompts.message}</p><Button variant="outline" onClick={() => void actions.refreshSystemPrompts(false)}>重试</Button></div>
      ) : filtered.length === 0 ? (
        <div className="system-prompt-empty"><FileText /><strong>该分类下暂无提示词</strong><Button onClick={() => setForm({ ...EMPTY_FORM, category: category === "全部" ? "软件开发" : category })}>添加提示词</Button></div>
      ) : (
        <div className="system-prompt-grid">
          {filtered.map((item) => <PromptCard key={item.id} actions={actions} item={item} active={prompts.activePromptId === item.id && prompts.managed} mode={activeMode ?? "preserve"} onEdit={edit} />)}
        </div>
      )}

      {form ? (
        <div className="system-prompt-modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setForm(null); }}>
          <section className="system-prompt-modal" role="dialog" aria-modal="true" aria-labelledby="prompt-editor-title">
            <header><div><small>CUSTOM PROMPT</small><h2 id="prompt-editor-title">{form.id ? "编辑提示词" : "添加提示词"}</h2></div><button type="button" title="关闭" onClick={() => setForm(null)}><X /></button></header>
            <div className="system-prompt-form-grid">
              <label><span>提示词名称</span><input value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} placeholder="例如：代码审查专家" /></label>
              <label><span>文件名</span><input value={form.filename} onChange={(e) => setForm({ ...form, filename: e.target.value })} placeholder="code-review.md" /></label>
              <label><span>分类</span><input value={form.category} onChange={(e) => setForm({ ...form, category: e.target.value })} placeholder="软件开发" /></label>
              <label><span>简介</span><input value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="卡片中显示的简短说明" /></label>
              <label className="is-wide"><span>提示词内容 <small>Markdown</small></span><textarea value={form.content} onChange={(e) => setForm({ ...form, content: e.target.value })} placeholder="在此输入提示词内容..." /></label>
            </div>
            <footer><Button variant="outline" onClick={() => setForm(null)}>取消</Button><Button disabled={!form.title.trim() || !form.content.trim()} onClick={() => void save()}>保存</Button></footer>
          </section>
        </div>
      ) : null}

      {syncOpen ? (
        <div className="system-prompt-modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setSyncOpen(false); }}>
          <section className="system-prompt-modal is-compact" role="dialog" aria-modal="true" aria-labelledby="prompt-sync-title">
            <header><div><small>GITHUB MARKDOWN</small><h2 id="prompt-sync-title">同步 GitHub 模板</h2></div><button type="button" title="关闭" onClick={() => setSyncOpen(false)}><X /></button></header>
            <label className="system-prompt-url"><span>Raw Markdown HTTPS 地址</span><input value={syncUrl} onChange={(e) => setSyncUrl(e.target.value)} placeholder="https://raw.githubusercontent.com/.../prompt.md" /></label>
            <footer><Button variant="outline" onClick={() => setSyncOpen(false)}>取消</Button><Button disabled={!syncUrl.trim()} onClick={async () => { const result = await actions.syncSystemPromptUrl(syncUrl); if (result && statusOk(result.status)) { setSyncOpen(false); setSyncUrl(""); } }}>同步</Button></footer>
          </section>
        </div>
      ) : null}

    </section>
  );
}
