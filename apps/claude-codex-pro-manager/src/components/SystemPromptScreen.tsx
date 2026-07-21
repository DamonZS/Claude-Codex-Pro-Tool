import { useEffect, useMemo, useState } from "react";
import { ArrowRightLeft, Check, CircleHelp, CirclePlus, FileText, Github, LoaderCircle, Pencil, Plus, RefreshCw, Trash2, Upload, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import guideMarkdown from "@/content/ccp-deepseek-guide.md?raw";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type { SaveSystemPromptRequest, SystemPromptItem, SystemPromptMode, SystemPromptResult } from "@/types";

type Props = { actions: AppActions; prompts: SystemPromptResult | null };

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

export function SystemPromptScreen({ actions, prompts }: Props) {
  const [mode, setMode] = useState<SystemPromptMode>("preserve");
  const [category, setCategory] = useState("全部");
  const [form, setForm] = useState<SaveSystemPromptRequest | null>(null);
  const [syncOpen, setSyncOpen] = useState(false);
  const [guideOpen, setGuideOpen] = useState(false);
  const [syncUrl, setSyncUrl] = useState("");
  const items = prompts?.prompts ?? [];
  const categories = useMemo(() => ["全部", ...Array.from(new Set(items.map((item) => item.category))).sort((a, b) => a.localeCompare(b, "zh-CN"))], [items]);
  const filtered = category === "全部" ? items : items.filter((item) => item.category === category);
  const failed = prompts !== null && !statusOk(prompts.status);
  const activeMode = prompts?.mode;

  useEffect(() => {
    if (activeMode) setMode(activeMode);
  }, [activeMode]);

  const edit = (item: SystemPromptItem) => setForm({
    id: item.id, title: item.title, filename: item.filename, description: item.description, category: item.category, content: item.content,
  });
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

      <div className="system-prompt-status-panel">
        <div className="system-prompt-active-state">
          <small>当前状态</small>
          <strong className={prompts?.managed ? "is-active" : ""}>
            <span className="system-prompt-dot" />
            <span>{prompts?.activeTitle || (prompts?.activePath ? "外部提示词" : "未启用提示词")}</span>
            {prompts?.managed && activeMode ? <em>{activeMode === "preserve" ? "保留指令文件" : "替换指令文件"}</em> : null}
          </strong>
          <p>{prompts?.externallyModified ? "Codex 配置已被其他程序修改，CCP 未执行覆盖。" : prompts?.managed ? `当前通过 model_instructions_file 加载（${activeMode === "preserve" ? "保留原提示词" : "替换原提示词"}）。` : prompts?.activePath || "选择下方模板后启用。"}</p>
        </div>
        <div className="system-prompt-mode">
          <div><strong>启用方式 <CircleHelp aria-hidden="true" /></strong><p>点击模板开关时，使用这里选择的方式。</p></div>
          <div className="system-prompt-segmented" role="group" aria-label="启用方式">
            <button type="button" className={mode === "preserve" ? "is-selected" : ""} onClick={() => setMode("preserve")}><CirclePlus aria-hidden="true" />保留原提示词</button>
            <button type="button" className={mode === "replace" ? "is-selected" : ""} onClick={() => setMode("replace")}><ArrowRightLeft aria-hidden="true" />替换原提示词</button>
          </div>
        </div>
        <Button className="system-prompt-guide-button" variant="outline" onClick={() => setGuideOpen(true)}>
          使用方式
        </Button>
      </div>

      <div className="system-prompt-filter-row">
        <div className="system-prompt-categories" role="tablist" aria-label="提示词分类">
          {categories.map((name) => <button key={name} type="button" role="tab" aria-selected={category === name} className={category === name ? "is-selected" : ""} onClick={() => setCategory(name)}>{name}</button>)}
        </div>
        <Button variant="ghost" onClick={() => void actions.refreshSystemPrompts(false)}><RefreshCw aria-hidden="true" />刷新</Button>
      </div>

      {prompts === null ? (
        <div className="system-prompt-empty"><LoaderCircle className="spin" /><strong>正在加载系统提示词</strong></div>
      ) : failed ? (
        <div className="system-prompt-empty is-error"><strong>系统提示词加载失败</strong><p>{prompts.message}</p><Button variant="outline" onClick={() => void actions.refreshSystemPrompts(false)}>重试</Button></div>
      ) : filtered.length === 0 ? (
        <div className="system-prompt-empty"><FileText /><strong>该分类下暂无提示词</strong><Button onClick={() => setForm({ ...EMPTY_FORM, category: category === "全部" ? "软件开发" : category })}>添加提示词</Button></div>
      ) : (
        <div className="system-prompt-grid">
          {filtered.map((item) => <PromptCard key={item.id} actions={actions} item={item} active={prompts.activePromptId === item.id && prompts.managed} mode={mode} onEdit={edit} />)}
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

      {guideOpen ? (
        <div className="system-prompt-modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setGuideOpen(false); }}>
          <section className="system-prompt-modal system-prompt-guide-modal" role="dialog" aria-modal="true" aria-labelledby="prompt-guide-title">
            <header>
              <div><small>CCP GUIDE</small><h2 id="prompt-guide-title">使用方式</h2></div>
              <button type="button" title="关闭" onClick={() => setGuideOpen(false)}><X /></button>
            </header>
            <pre className="system-prompt-guide-content">{guideMarkdown}</pre>
          </section>
        </div>
      ) : null}
    </section>
  );
}
