import {
  Check,
  FolderOpen,
  Image as ImageIcon,
  LoaderCircle,
  Palette,
  Plus,
  RefreshCw,
  RotateCcw,
  Upload,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type {
  CodexThemeListResult,
  CodexThemeOperationState,
  CodexThemeSummary,
} from "@/types";

type CodexThemeCenterScreenProps = {
  actions: AppActions;
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
  if (operation.kind === "restore") return "正在恢复默认主题";
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
  theme,
}: {
  actions: AppActions;
  operation: CodexThemeOperationState | null;
  theme: CodexThemeSummary;
}) {
  const disabled = Boolean(operation);
  const isApplying = operation?.themeId === theme.id;
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
        <span className={theme.current ? "is-current" : ""}>
          {isApplying ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
          {theme.current ? "已启用" : label}
        </span>
      </footer>
    </article>
  );
}

export function CodexThemeCenterScreen({ actions, operation, themes }: CodexThemeCenterScreenProps) {
  const themeItems = themes?.themes ?? [];
  const orderedThemes = [...themeItems].sort((left, right) => {
    if (left.builtin !== right.builtin) return left.builtin ? -1 : 1;
    if (left.current !== right.current) return left.current ? -1 : 1;
    return left.name.localeCompare(right.name, "zh-CN");
  });
  const loading = themes === null;
  const failed = themes !== null && !statusOk(themes.status);

  return (
    <section className="ops-screen codex-theme-screen" aria-labelledby="codex-theme-title">
      <header className="ops-page-heading codex-theme-heading">
        <div>
          <p className="codex-theme-eyebrow">Codex / 外观</p>
          <h1 id="codex-theme-title">主题中心</h1>
          <p>管理 Codex 的视觉主题。主题文件会先验证，再原子替换；应用后重启 Codex 生效。</p>
        </div>
        <div className="codex-theme-toolbar">
          <Button
            type="button"
            variant="outline"
            disabled={Boolean(operation)}
            onClick={() => void actions.refreshCodexThemes(false)}
          >
            <RefreshCw aria-hidden="true" />
            刷新
          </Button>
          <details className="codex-theme-import-menu">
            <summary className="button button-default" aria-label="新建主题">
              <Plus aria-hidden="true" />
              新建主题
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
        </div>
      </header>

      <div className="codex-theme-status-row" role="status" aria-live="polite">
        <span className="codex-theme-count"><Palette aria-hidden="true" />{themeItems.length} 个主题</span>
        <span>默认主题固定在第一张卡片</span>
        {operation ? <span className="codex-theme-operation"><LoaderCircle className="spin" aria-hidden="true" />{operationLabel(operation)}</span> : null}
      </div>

      {loading ? (
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
          <span>点击“新建主题”导入 ZIP 主题包或主题目录。</span>
        </div>
      ) : (
        <div className="codex-theme-grid">
          {orderedThemes.map((theme) => (
            <ThemeCard key={theme.id} actions={actions} operation={operation} theme={theme} />
          ))}
        </div>
      )}

      <aside className="codex-theme-help">
        <RotateCcw aria-hidden="true" />
        <span><strong>可随时回滚。</strong> 默认主题不会被删除，主题更新会保留上一版本，应用失败时自动恢复。</span>
      </aside>
    </section>
  );
}
