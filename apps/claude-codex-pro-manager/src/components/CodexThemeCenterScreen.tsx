import {
  BookOpen,
  Check,
  Download,
  FolderOpen,
  Image as ImageIcon,
  ImagePlus,
  LoaderCircle,
  Palette,
  Pencil,
  RefreshCw,
  RotateCcw,
  Trash2,
  Upload,
  WandSparkles,
} from "lucide-react";
import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { CodexThemeDiyDialog } from "@/components/CodexThemeDiyDialog";
import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type {
  CodexManagerBackgroundItem,
  CodexManagerBackgroundLibraryResult,
  CodexOfficialTheme,
  CodexThemeBackgroundResult,
  CodexThemeListResult,
  CodexThemeOperationState,
  CodexThemeSummary,
} from "@/types";

type ThemeDownloadMenuProps = {
  actions: AppActions;
  installedThemeIds: Set<string>;
  officialThemes: CodexOfficialTheme[];
  operation: CodexThemeOperationState | null;
};

function ThemeDownloadMenu({ actions, installedThemeIds, officialThemes, operation }: ThemeDownloadMenuProps) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<CSSProperties>({});
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDialogElement>(null);

  const updatePosition = useCallback(() => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({
      top: Math.min(rect.bottom + 6, window.innerHeight - 24),
      right: Math.max(12, window.innerWidth - rect.right),
    });
  }, []);

  useEffect(() => {
    if (!open) return;
    updatePosition();
    const dialog = menuRef.current;
    if (dialog && !dialog.open) dialog.showModal();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
      if (dialog?.open) dialog.close();
    };
  }, [open, updatePosition]);

  return (
    <>
      <Button
        ref={buttonRef}
        type="button"
        variant="outline"
        className="codex-theme-download-trigger"
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen((current) => !current)}
      >
        <Download aria-hidden="true" />
        下载主题
      </Button>
      {open ? createPortal(
        <dialog
          ref={menuRef}
          className="codex-theme-import-options codex-theme-download-options codex-theme-download-portal"
          style={position}
          aria-label="下载官方主题"
          onCancel={(event) => {
            event.preventDefault();
            setOpen(false);
          }}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setOpen(false);
          }}
        >
          {officialThemes.map((theme) => {
            const installed = installedThemeIds.has(theme.id);
            return (
              <button
                type="button"
                role="menuitem"
                key={theme.id}
                disabled={Boolean(operation) || installed}
                onClick={() => {
                  setOpen(false);
                  void actions.downloadCodexTheme(theme.id);
                }}
              >
                {installed ? <Check aria-hidden="true" /> : <Download aria-hidden="true" />}
                <span>
                  <strong>{theme.name}</strong>
                  <small>{installed ? "已安装" : theme.id}</small>
                </span>
              </button>
            );
          })}
        </dialog>,
        document.body,
      ) : null}
    </>
  );
}

type CodexThemeCenterScreenProps = {
  actions: AppActions;
  background: CodexThemeBackgroundResult | null;
  managerBackgrounds: CodexManagerBackgroundLibraryResult | null;
  operation: CodexThemeOperationState | null;
  themes: CodexThemeListResult | null;
};

function formatThemeDate(value: number) {
  if (!value) return "内置主题";
  const date = new Date(value * 1000);
  return Number.isNaN(date.getTime()) ? "最近更新" : date.toLocaleDateString("zh-CN");
}

function operationLabel(operation: CodexThemeOperationState | null) {
  if (!operation) return "";
  if (operation.kind === "import") return "正在验证并保存主题";
  if (operation.kind === "download") return "正在从 GitHub 下载并验证主题";
  if (operation.kind === "delete") return "正在安全删除主题";
  if (operation.kind === "diy-save") return "正在生成并保存 DIY 主题";
  if (operation.kind === "restore") return "正在恢复默认主题";
  if (operation.kind === "background") return "正在校验并应用背景";
  if (operation.kind === "background-apply") return "正在切换 CCP 背景";
  if (operation.kind === "background-delete") return "正在删除 CCP 背景";
  if (operation.kind === "clear-background") return "正在恢复主题背景";
  return "正在应用主题";
}

function ThemePreview({ theme }: { theme: CodexThemeSummary }) {
  if (theme.preview_data_uri) {
    return <img className="codex-theme-preview-image" src={theme.preview_data_uri} alt={`${theme.name} 预览`} />;
  }
  return (
    <div className={`codex-theme-preview-fallback${theme.builtin ? " is-default" : ""}`} aria-hidden="true">
      <Palette />
      <span>{theme.builtin ? "CODEX" : "THEME"}</span>
    </div>
  );
}

function ThemeCard({
  actions,
  operation,
  onEdit,
  theme,
}: {
  actions: AppActions;
  operation: CodexThemeOperationState | null;
  onEdit: (theme: CodexThemeSummary) => void;
  theme: CodexThemeSummary;
}) {
  const disabled = Boolean(operation);
  const isApplying = operation?.kind === "apply" && operation.themeId === theme.id;
  const isDeleting = operation?.kind === "delete" && operation.themeId === theme.id;
  const label = theme.current ? "当前主题" : theme.builtin ? "恢复默认" : "应用主题";

  return (
    <article className={`codex-theme-card${theme.current ? " is-current" : ""}`}>
      <button
        type="button"
        className="codex-theme-card-main"
        disabled={disabled}
        aria-pressed={theme.current}
        onClick={() => {
          if (theme.builtin) {
            void actions.restoreCodexDefaultTheme();
          } else {
            void actions.applyCodexTheme(theme.id);
          }
        }}
      >
        <span className="codex-theme-preview">
          <ThemePreview theme={theme} />
          {theme.current ? <span className="codex-theme-current-badge"><Check aria-hidden="true" />当前</span> : null}
        </span>
        <span className="codex-theme-card-copy">
          <span className="codex-theme-card-title-row">
            <strong>{theme.name}</strong>
            <small>{theme.version}</small>
          </span>
          <span className="codex-theme-author">{theme.author || "未标注作者"}</span>
          <span className="codex-theme-description">{theme.description || "暂无主题描述"}</span>
        </span>
      </button>
      <footer className="codex-theme-card-footer">
        <span>{theme.builtin ? "Codex 内置" : `导入于 ${formatThemeDate(theme.imported_at)}`}</span>
        <span className="codex-theme-card-footer-actions">
          <span className={theme.current ? "is-current" : ""}>
            {isApplying ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
            {theme.current ? "已启用" : label}
          </span>
          {theme.diy ? (
            <button
              type="button"
              className="codex-theme-card-icon-button"
              disabled={disabled}
              title={`编辑 ${theme.name}`}
              aria-label={`编辑 DIY 主题 ${theme.name}`}
              onClick={() => onEdit(theme)}
            >
              <Pencil aria-hidden="true" />
            </button>
          ) : null}
          {!theme.builtin ? (
            <button
              type="button"
              className="codex-theme-card-icon-button is-danger"
              disabled={disabled || theme.current}
              title={theme.current ? "请先恢复默认主题或切换到其他主题" : `删除 ${theme.name}`}
              aria-label={`删除主题 ${theme.name}`}
              onClick={() => void actions.deleteCodexTheme(theme.id)}
            >
              {isDeleting ? <LoaderCircle className="spin" aria-hidden="true" /> : <Trash2 aria-hidden="true" />}
            </button>
          ) : null}
        </span>
      </footer>
    </article>
  );
}

function ManagerBackgroundCard({
  actions,
  background,
  current,
  item,
  operation,
}: {
  actions: AppActions;
  background: CodexThemeBackgroundResult | null;
  current: boolean;
  item: CodexManagerBackgroundItem | null;
  operation: CodexThemeOperationState | null;
}) {
  const isDefault = item === null;
  const busy = Boolean(operation);
  const applying = operation?.kind === (isDefault ? "clear-background" : "background-apply")
    && (isDefault || operation.themeId === item.id);
  const deleting = !isDefault && operation?.kind === "background-delete" && operation.themeId === item.id;
  const preview = isDefault ? (!background?.user_override ? background?.data_uri : null) : item.preview_data_uri;

  return (
    <article className={`ccp-background-card${current ? " is-current" : ""}`}>
      <button
        type="button"
        className="ccp-background-card-main"
        disabled={busy || current}
        aria-pressed={current}
        onClick={() => {
          if (isDefault) void actions.clearCodexManagerBackground();
          else void actions.applyCodexManagerBackground(item.id);
        }}
      >
        <span className={`ccp-background-card-preview${preview ? " has-image" : " is-default"}`}>
          {preview ? <img src={preview} alt={isDefault ? "CCP 默认外观预览" : `${item.file_name} 预览`} /> : <span><ImageIcon aria-hidden="true" />CCP</span>}
          {current ? <span className="ccp-background-current-badge"><Check aria-hidden="true" />正在使用</span> : null}
        </span>
        <span className="ccp-background-card-copy">
          <strong>{isDefault ? "CCP 默认背景" : item.file_name}</strong>
          <small>{isDefault ? "未启用图库背景" : `${item.width} × ${item.height} · ${item.mime_type.replace("image/", "").toUpperCase()}`}</small>
        </span>
      </button>
      <footer>
        <span>{current ? "当前背景" : isDefault ? "恢复后仍保留背景图库" : `保存于 ${formatThemeDate(item.updated_at)}`}</span>
        <span>
          {applying ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
          {!current ? <b>{isDefault ? "恢复默认" : "应用背景"}</b> : null}
          {!isDefault ? (
            <button
              type="button"
              className="codex-theme-card-icon-button is-danger"
              disabled={busy || current}
              title={current ? "请先切换到其他背景" : `删除 ${item.file_name}`}
              aria-label={`删除 CCP 背景 ${item.file_name}`}
              onClick={() => void actions.deleteCodexManagerBackground(item.id)}
            >
              {deleting ? <LoaderCircle className="spin" aria-hidden="true" /> : <Trash2 aria-hidden="true" />}
            </button>
          ) : null}
        </span>
      </footer>
    </article>
  );
}

export function CodexThemeCenterScreen({ actions, background, managerBackgrounds, operation, themes }: CodexThemeCenterScreenProps) {
  const [view, setView] = useState<"ccp" | "codex">("ccp");
  const [diyTheme, setDiyTheme] = useState<CodexThemeSummary | null | undefined>(undefined);
  const themeItems = themes?.themes ?? [];
  const officialThemes = themes?.official_themes ?? [];
  const installedThemeIds = new Set(themeItems.map((theme) => theme.id));
  const orderedThemes = [...themeItems].sort((left, right) => {
    if (left.builtin !== right.builtin) return left.builtin ? -1 : 1;
    if (left.current !== right.current) return left.current ? -1 : 1;
    return left.name.localeCompare(right.name, "zh-CN");
  });
  const loading = themes === null;
  const failed = themes !== null && !statusOk(themes.status);
  const managerItems = managerBackgrounds?.items ?? [];
  const managerLoading = managerBackgrounds === null;
  const managerFailed = managerBackgrounds !== null && !statusOk(managerBackgrounds.status);
  const defaultManagerBackgroundCurrent = managerBackgrounds
    ? managerBackgrounds.current_background_id === null
    : !background?.user_override;

  return (
    <section className="ops-screen codex-theme-screen" aria-labelledby="codex-theme-title">
      <header className="ops-page-heading codex-theme-heading">
        <div>
          <p className="codex-theme-eyebrow">外观中心 / {view === "ccp" ? "CCP" : "CODEX"}</p>
          <h1 id="codex-theme-title">{view === "ccp" ? "CCP 外观" : "Codex 主题"}</h1>
          <p>{view === "ccp" ? "管理 CCP 本机背景图库，不影响 Codex。" : "管理 Codex 注入主题，应用后重启 Codex 生效。"}</p>
        </div>
        <div className="codex-theme-toolbar">
          <Button
            type="button"
            variant="outline"
            disabled={Boolean(operation)}
            onClick={() => void Promise.all([actions.refreshCodexThemes(false), actions.refreshCodexManagerBackgrounds(false)])}
          >
            <RefreshCw aria-hidden="true" />
            刷新
          </Button>
          {view === "ccp" ? (
            <Button type="button" disabled={Boolean(operation)} onClick={() => void actions.setCodexManagerBackground()}>
              {operation?.kind === "background" ? <LoaderCircle className="spin" aria-hidden="true" /> : <ImagePlus aria-hidden="true" />}
              添加背景
            </Button>
          ) : (
            <>
              <Button type="button" variant="outline" disabled={Boolean(operation)} onClick={() => setDiyTheme(null)}>
                <WandSparkles aria-hidden="true" />
                DIY 主题
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool/blob/main/Theme/README.md#build-your-own-codex-theme")}
              >
                <BookOpen aria-hidden="true" />
                制作指南
              </Button>
              <ThemeDownloadMenu actions={actions} installedThemeIds={installedThemeIds} officialThemes={officialThemes} operation={operation} />
              <details className="codex-theme-import-menu">
            <summary className="button button-default" aria-label="导入主题">
              <Upload aria-hidden="true" />
              导入主题
            </summary>
            <div className="codex-theme-import-options">
              <button type="button" disabled={Boolean(operation)} onClick={() => void actions.importCodexTheme("zip")}>
                <Upload aria-hidden="true" />
                导入 ZIP 主题包
              </button>
              <button type="button" disabled={Boolean(operation)} onClick={() => void actions.importCodexTheme("directory")}>
                <FolderOpen aria-hidden="true" />
                导入主题目录
              </button>
            </div>
              </details>
            </>
          )}
        </div>
      </header>

      <div className="codex-theme-domain-switch" role="tablist" aria-label="外观作用范围">
        <button type="button" role="tab" aria-controls="ccp-appearance-panel" aria-selected={view === "ccp"} className={view === "ccp" ? "is-active" : ""} onClick={() => setView("ccp")}>
          <ImageIcon aria-hidden="true" />CCP 外观
        </button>
        <button type="button" role="tab" aria-controls="codex-theme-panel" aria-selected={view === "codex"} className={view === "codex" ? "is-active" : ""} onClick={() => setView("codex")}>
          <Palette aria-hidden="true" />Codex 主题
        </button>
      </div>

      <div className="codex-theme-status-row" role="status" aria-live="polite">
        <span className="codex-theme-count">
          {view === "ccp" ? <ImageIcon aria-hidden="true" /> : <Palette aria-hidden="true" />}
          {view === "ccp" ? `${managerItems.length} 张已保存背景` : `${themeItems.length} 个 Codex 主题`}
        </span>
        <span>{view === "ccp" ? "默认外观固定在第一张卡片" : "Codex 默认主题固定在第一张卡片"}</span>
        {operation ? <span className="codex-theme-operation"><LoaderCircle className="spin" aria-hidden="true" />{operationLabel(operation)}</span> : null}
      </div>

      {view === "ccp" ? (
        managerLoading ? (
          <div className="codex-theme-state" role="status"><LoaderCircle className="spin" aria-hidden="true" /><strong>正在加载 CCP 背景图库</strong><span>读取本地高清背景，请稍候。</span></div>
        ) : managerFailed ? (
          <div className="codex-theme-state is-error" role="alert">
            <strong>CCP 背景图库加载失败</strong>
            <span>{managerBackgrounds.message || "请刷新后重试。"}</span>
            <Button type="button" variant="outline" onClick={() => void actions.refreshCodexManagerBackgrounds(false)}><RefreshCw aria-hidden="true" />重试</Button>
          </div>
        ) : (
          <div id="ccp-appearance-panel" className="ccp-background-grid" role="tabpanel" aria-label="CCP 背景图库">
            <ManagerBackgroundCard actions={actions} background={background} current={defaultManagerBackgroundCurrent} item={null} operation={operation} />
            {managerItems.map((item) => (
              <ManagerBackgroundCard key={item.id} actions={actions} background={background} current={item.current} item={item} operation={operation} />
            ))}
            <button type="button" className="ccp-background-add-card" disabled={Boolean(operation)} onClick={() => void actions.setCodexManagerBackground()}>
              <ImagePlus aria-hidden="true" />
              <strong>添加高清背景</strong>
              <span>PNG、JPEG 或 WebP<br />长边 ≥ 1280，短边 ≥ 720</span>
            </button>
          </div>
        )
      ) : null}

      {view === "codex" && (loading ? (
        <div className="codex-theme-state" role="status">
          <LoaderCircle className="spin" aria-hidden="true" />
          <strong>正在加载主题</strong>
          <span>读取本地主题库，请稍候。</span>
        </div>
      ) : failed ? (
        <div className="codex-theme-state is-error" role="alert">
          <strong>主题库加载失败</strong>
          <span>{themes.message || "请刷新后重试。"}</span>
          <Button type="button" variant="outline" onClick={() => void actions.refreshCodexThemes(false)}>
            <RefreshCw aria-hidden="true" />
            重试
          </Button>
        </div>
      ) : orderedThemes.length === 0 ? (
        <div className="codex-theme-state">
          <ImageIcon aria-hidden="true" />
          <strong>还没有可用主题</strong>
          <span>点击“DIY 主题”可视化创建，或通过“导入主题”添加现有主题包。</span>
        </div>
      ) : (
        <div id="codex-theme-panel" className="codex-theme-grid" role="tabpanel" aria-label="Codex 主题库">
          {orderedThemes.map((theme) => (
            <ThemeCard key={theme.id} actions={actions} operation={operation} onEdit={setDiyTheme} theme={theme} />
          ))}
        </div>
      ))}

      <aside className="codex-theme-help">
        <RotateCcw aria-hidden="true" />
        <span><strong>{view === "ccp" ? "背景不会被覆盖。" : "可随时回滚。"}</strong> {view === "ccp" ? "恢复默认只取消启用，图库中的高清背景仍会保留。" : "默认主题不会被删除，主题更新会保留上一版本，应用失败时自动恢复。"}</span>
      </aside>

      {diyTheme !== undefined ? (
        <CodexThemeDiyDialog
          key={diyTheme?.id ?? "new"}
          actions={actions}
          operation={operation}
          theme={diyTheme}
          onClose={() => setDiyTheme(undefined)}
        />
      ) : null}
    </section>
  );
}
