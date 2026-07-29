import {
  ArrowUp,
  ChevronDown,
  Folder,
  GitBranch,
  ImagePlus,
  LayoutDashboard,
  Laptop,
  LoaderCircle,
  Maximize2,
  Mic,
  Palette,
  PanelTop,
  Plus,
  RotateCcw,
  Search,
  Settings2,
  SquarePen,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { type CSSProperties, type KeyboardEvent as ReactKeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { Button } from "@/components/ui/button";
import type { AppActions } from "@/lib/actions";
import { statusOk } from "@/lib/helpers";
import type {
  CodexThemeDiyAutomaticPalette,
  CodexThemeDiyImageLayout,
  CodexThemeDiyInput,
  CodexThemeDiySettings,
  CodexThemeOperationState,
  CodexThemeSummary,
} from "@/types";

type CodexThemeDiyDialogProps = {
  actions: AppActions;
  operation: CodexThemeOperationState | null;
  theme: CodexThemeSummary | null;
  onClose: () => void;
};

type DiyDraft = {
  name: string;
  description: string;
  settings: CodexThemeDiySettings;
};

type BackgroundChoice = "none" | "keep" | "replace" | "remove";
type DiyTextTone = "light" | "dark";

const DIY_TEXT_COLORS: Record<DiyTextTone, string> = {
  light: "#F3F5F7",
  dark: "#17191C",
};

const DEFAULT_AUTOMATIC_PALETTE: CodexThemeDiyAutomaticPalette = {
  mode: "dark",
  accent_color: "#0A84FF",
  background_color: "#111418",
  surface_color: "#20252B",
  text_color: "#F3F5F7",
};

function defaultSettings(): CodexThemeDiySettings {
  return {
    ...DEFAULT_AUTOMATIC_PALETTE,
    glass_opacity: 26,
    blur_px: 24,
    radius_px: 8,
    font_scale_percent: 100,
    density: "comfortable",
    image_layout: "card",
    background_file_name: null,
  };
}

function draftForTheme(theme: CodexThemeSummary | null): DiyDraft {
  const settings = { ...defaultSettings(), ...(theme?.diy ?? {}) };
  settings.glass_opacity = Math.min(90, Math.max(8, settings.glass_opacity));
  settings.blur_px = Math.min(48, Math.max(0, settings.blur_px));
  settings.font_scale_percent = 100;
  settings.density = "comfortable";
  settings.text_color = DIY_TEXT_COLORS[diyTextTone(settings.text_color)];
  return {
    name: theme?.name ?? "我的 Codex 主题",
    description: theme?.description ?? "",
    settings,
  };
}

function settingsWithAutomaticPalette(
  current: CodexThemeDiySettings,
  palette: CodexThemeDiyAutomaticPalette,
): CodexThemeDiySettings {
  return {
    ...current,
    ...palette,
    font_scale_percent: 100,
    density: "comfortable",
  };
}

function diyTextTone(color: string): DiyTextTone {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color);
  if (!match) return "light";
  const luma = Number.parseInt(match[1], 16) * 0.2126
    + Number.parseInt(match[2], 16) * 0.7152
    + Number.parseInt(match[3], 16) * 0.0722;
  return luma >= 128 ? "light" : "dark";
}

function fileNameFromPath(path: string) {
  return path.replace(/\\/g, "/").split("/").filter(Boolean).at(-1) || "已选择背景图";
}

function validateDraft(draft: DiyDraft) {
  const nameLength = draft.name.trim().length;
  if (nameLength < 1 || nameLength > 80) return "主题名称需要填写 1 至 80 个字符。";
  if (draft.description.length > 400) return "主题说明不能超过 400 个字符。";
  return "";
}

const IMAGE_LAYOUT_LABELS: Record<CodexThemeDiyImageLayout, string> = {
  fullscreen: "全屏透明背景",
  banner: "上方长条",
  card: "中央大卡片",
};

const IMAGE_LAYOUT_DESCRIPTIONS: Record<CodexThemeDiyImageLayout, string> = {
  fullscreen: "图片以原始亮度铺满 Codex 画布，不叠加全屏蒙层，中央保留原生图标。",
  banner: "图片以原始亮度显示为首页上方长条，界面色调自动与图片协调。",
  card: "图片以原始亮度显示为中央大卡片并保留完整比例，界面色调自动协调。",
};

function CodexDiyPreview({
  draft,
  backgroundUrl,
}: {
  draft: DiyDraft;
  backgroundUrl: string | null;
}) {
  const { settings } = draft;
  const heroImageUrl = settings.image_layout === "fullscreen" ? null : backgroundUrl;
  const style = {
    "--diy-primary": settings.accent_color,
    "--diy-background": settings.background_color,
    "--diy-surface": settings.surface_color,
    "--diy-text": settings.text_color,
    "--diy-text-shadow": diyTextTone(settings.text_color) === "dark"
      ? "0 1px 2px rgb(255 255 255 / 0.72)"
      : "0 1px 2px rgb(0 0 0 / 0.68)",
  } as CSSProperties;

  return (
    <div className={`codex-diy-preview is-${settings.mode} is-${settings.density} is-layout-${settings.image_layout}`} style={style}>
      {backgroundUrl && settings.image_layout === "fullscreen" ? <img className="codex-diy-preview-backdrop-art" src={backgroundUrl} alt="" aria-hidden="true" /> : null}
      <aside className="codex-diy-preview-sidebar">
        <div className="codex-diy-preview-brand">
          <strong>Codex</strong><ChevronDown aria-hidden="true" /><Search className="codex-diy-preview-search" aria-hidden="true" />
        </div>
        <nav aria-label="Codex 预览导航">
          <span className="is-active"><SquarePen aria-hidden="true" />新建任务</span>
          <div className="codex-diy-preview-project">
            <span><Folder aria-hidden="true" />示例项目</span>
            <small>继续处理当前任务</small>
          </div>
          <div className="codex-diy-preview-project is-current">
            <span><Folder aria-hidden="true" />当前工作区</span>
            <small>检查主题预览</small>
            <small>运行项目验证</small>
          </div>
          <div className="codex-diy-preview-project">
            <span><Folder aria-hidden="true" />本地项目</span>
            <small>查看最近任务</small>
          </div>
        </nav>
        <span className="codex-diy-preview-settings"><Settings2 aria-hidden="true" />My Codex</span>
      </aside>
      <div className="codex-diy-preview-workspace">
        <span className="codex-diy-preview-window-controls" aria-hidden="true"><i /><i /></span>
        <main className="codex-diy-preview-home">
          <section className={`codex-diy-preview-hero ${heroImageUrl ? "has-image" : ""}`}>
            {heroImageUrl ? (
              <img className="codex-diy-preview-hero-image" src={heroImageUrl} alt="主题图片预览" />
            ) : <span className="codex-diy-preview-avatar"><Sparkles aria-hidden="true" /></span>}
          </section>
          <strong className="codex-diy-preview-home-title">What should we build?</strong>
        </main>
        <div className="codex-diy-preview-composer">
          <div className="codex-diy-preview-context"><span><Folder aria-hidden="true" />当前工作区</span><span><Laptop aria-hidden="true" />本地</span><span><GitBranch aria-hidden="true" />main</span></div>
          <div className="codex-diy-preview-input">
            <span>随心输入</span>
            <div><Plus aria-hidden="true" /><small>完全访问</small><Mic aria-hidden="true" /><button type="button" aria-label="发送预览消息"><ArrowUp aria-hidden="true" /></button></div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function CodexThemeDiyDialog({ actions, operation, theme, onClose }: CodexThemeDiyDialogProps) {
  const initialDraft = useMemo(() => draftForTheme(theme), [theme]);
  const [draft, setDraft] = useState<DiyDraft>(initialDraft);
  const hasExistingBackground = Boolean(theme?.diy?.background_file_name);
  const [backgroundChoice, setBackgroundChoice] = useState<BackgroundChoice>(hasExistingBackground ? "keep" : "none");
  const [backgroundPath, setBackgroundPath] = useState<string | null>(null);
  const [selectedBackgroundUrl, setSelectedBackgroundUrl] = useState<string | null>(null);
  const [existingBackgroundUrl, setExistingBackgroundUrl] = useState<string | null>(null);
  const [backgroundPreviewLoading, setBackgroundPreviewLoading] = useState(hasExistingBackground);
  const [backgroundLoadError, setBackgroundLoadError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState("");
  const dialogRef = useRef<HTMLElement>(null);
  const initialBackgroundChoice = hasExistingBackground ? "keep" : "none";
  const dirty = JSON.stringify(draft) !== JSON.stringify(initialDraft)
    || backgroundChoice !== initialBackgroundChoice
    || Boolean(backgroundPath);
  const saving = submitting || operation?.kind === "diy-save";
  const controlsDisabled = saving || backgroundPreviewLoading;
  const validationError = validateDraft(draft);
  const backgroundUrl = backgroundChoice === "replace"
    ? selectedBackgroundUrl
    : backgroundChoice === "keep"
      ? existingBackgroundUrl
      : null;
  const backgroundThumbnailUrl = backgroundUrl;

  const updateSetting = <K extends keyof CodexThemeDiySettings>(key: K, value: CodexThemeDiySettings[K]) => {
    setDraft((current) => ({ ...current, settings: { ...current.settings, [key]: value } }));
    setSubmitError("");
  };

  const requestClose = useCallback(() => {
    if (saving) return;
    if (dirty && !window.confirm("主题还有未保存的修改，确认关闭 DIY 工作台？")) return;
    onClose();
  }, [dirty, onClose, saving]);

  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") requestClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [requestClose]);

  useEffect(() => {
    if (!theme?.id || !hasExistingBackground) {
      setExistingBackgroundUrl(null);
      setBackgroundPreviewLoading(false);
      setBackgroundLoadError("");
      return;
    }
    let cancelled = false;
    setBackgroundPreviewLoading(true);
    setBackgroundLoadError("");
    void actions.loadCodexDiyThemeBackground(theme.id)
      .then((result) => {
        if (cancelled) return;
        if (result && statusOk(result.status) && result.data_uri) {
          setExistingBackgroundUrl(result.data_uri);
          setDraft((current) => ({
            ...current,
            settings: settingsWithAutomaticPalette(current.settings, result.automatic_palette),
          }));
          return;
        }
        setExistingBackgroundUrl(null);
        setBackgroundLoadError(result?.message || "原背景预览加载失败，保存时仍会保留原图。");
      })
      .catch((error) => {
        if (cancelled) return;
        setExistingBackgroundUrl(null);
        setBackgroundLoadError(error instanceof Error ? error.message : "原背景预览加载失败，保存时仍会保留原图。");
      })
      .finally(() => {
        if (!cancelled) setBackgroundPreviewLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [actions, hasExistingBackground, theme?.id]);

  const chooseBackground = async () => {
    setSubmitError("");
    try {
      const path = await actions.selectCodexDiyThemeBackground();
      if (!path) return;
      setBackgroundPreviewLoading(true);
      const preview = await actions.previewCodexDiyThemeBackground(path);
      if (!preview || !statusOk(preview.status) || !preview.data_uri) {
        setSubmitError(preview?.message || "背景图校验失败，请重新选择。");
        return;
      }
      setBackgroundPath(path);
      setSelectedBackgroundUrl(preview.data_uri);
      setBackgroundChoice("replace");
      setDraft((current) => ({
        ...current,
        settings: settingsWithAutomaticPalette(current.settings, preview.automatic_palette),
      }));
      setBackgroundLoadError("");
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : "背景图选择失败，请重试。");
    } finally {
      setBackgroundPreviewLoading(false);
    }
  };

  const removeBackground = () => {
    setBackgroundPath(null);
    setSelectedBackgroundUrl(null);
    setBackgroundChoice(hasExistingBackground ? "remove" : "none");
    setDraft((current) => ({
      ...current,
      settings: settingsWithAutomaticPalette(current.settings, DEFAULT_AUTOMATIC_PALETTE),
    }));
    setSubmitError("");
  };

  const keepBackground = () => {
    setBackgroundPath(null);
    setSelectedBackgroundUrl(null);
    setBackgroundChoice("keep");
    setSubmitError("");
  };

  const keepFocusInside = (event: ReactKeyboardEvent<HTMLElement>) => {
    if (event.key !== "Tab") return;
    const focusable = Array.from(dialogRef.current?.querySelectorAll<HTMLElement>(
      "button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
    ) ?? []).filter((element) => element.offsetParent !== null);
    const first = focusable[0];
    const last = focusable.at(-1);
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const save = async (applyAfterSave: boolean) => {
    const error = validateDraft(draft);
    if (error) {
      setSubmitError(error);
      return;
    }
    const input: CodexThemeDiyInput = {
      theme_id: theme?.id ?? null,
      expected_integrity_sha256: theme?.integrity_sha256 ?? null,
      name: draft.name.trim(),
      author: theme?.author || "CCP 用户",
      description: draft.description.trim(),
      settings: draft.settings,
      background_path: backgroundChoice === "replace" ? backgroundPath : null,
      remove_background: backgroundChoice === "remove",
    };
    setSubmitting(true);
    setSubmitError("");
    try {
      const result = await actions.saveCodexDiyTheme(input, applyAfterSave);
      if (result && (statusOk(result.status) || result.status === "partial")) {
        onClose();
        return;
      }
      setSubmitError(result?.message || "主题保存失败，请检查设置后重试。");
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : "主题保存失败，请重试。");
    } finally {
      setSubmitting(false);
    }
  };

  const portalTarget = document.querySelector<HTMLElement>(".ops-shell") ?? document.body;

  return createPortal(
    <div
      className="codex-diy-backdrop"
      role="presentation"
      onMouseDown={(event) => { if (event.currentTarget === event.target) requestClose(); }}
    >
      <section
        ref={dialogRef}
        className="codex-diy-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="codex-diy-title"
        onKeyDown={keepFocusInside}
      >
        <header className="codex-diy-dialog-header">
          <div>
            <span className="codex-diy-dialog-icon"><Palette aria-hidden="true" /></span>
            <div>
              <small>NO-CODE THEME STUDIO</small>
              <h2 id="codex-diy-title">{theme ? `编辑 ${theme.name}` : "DIY Codex 主题"}</h2>
            </div>
          </div>
          <button type="button" className="codex-diy-close" title="关闭" aria-label="关闭 DIY 主题工作台" disabled={saving} onClick={requestClose}>
            <X aria-hidden="true" />
          </button>
        </header>

        <div className="codex-diy-dialog-body">
          <form
            className="codex-diy-controls"
            aria-busy={controlsDisabled}
            inert={controlsDisabled ? true : undefined}
            onSubmit={(event) => event.preventDefault()}
          >
            <fieldset>
              <legend>主题信息</legend>
              <label className="codex-diy-field">
                <span>主题名称 <small>{draft.name.length}/80</small></span>
                <input
                  autoFocus
                  type="text"
                  maxLength={80}
                  value={draft.name}
                  placeholder="例如：暖阳主题"
                  aria-invalid={draft.name.trim().length < 1}
                  onChange={(event) => { setDraft({ ...draft, name: event.target.value }); setSubmitError(""); }}
                />
              </label>
              <label className="codex-diy-field">
                <span>主题说明 <small>{draft.description.length}/400</small></span>
                <textarea
                  rows={3}
                  maxLength={400}
                  value={draft.description}
                  placeholder="简要描述主题的视觉特点"
                  onChange={(event) => { setDraft({ ...draft, description: event.target.value }); setSubmitError(""); }}
                />
              </label>
            </fieldset>

            <fieldset>
              <legend>自动外观</legend>
              <div className="codex-diy-auto-appearance">
                <Sparkles aria-hidden="true" />
                <span>
                  <strong>提取图片主色调</strong>
                  <small>从图片生成主色、表面色和对比文字色，保持工作区清晰可读。</small>
                </span>
                <b>{draft.settings.mode === "dark" ? "深色 · 白字" : "浅色 · 黑字"}</b>
              </div>
              <div className="codex-diy-segment-field codex-diy-text-color-field">
                <span>字体颜色</span>
                <b>{draft.settings.mode === "dark" ? "自动白字" : "自动黑字"}</b>
              </div>
            </fieldset>

            <fieldset>
              <legend>背景图</legend>
              <div className="codex-diy-background-control">
                <div className={backgroundThumbnailUrl ? "has-image" : ""} style={{ backgroundImage: backgroundThumbnailUrl ? `url(${JSON.stringify(backgroundThumbnailUrl)})` : undefined }} aria-busy={backgroundPreviewLoading}>
                  {backgroundPreviewLoading ? <LoaderCircle className="spin" aria-hidden="true" /> : backgroundThumbnailUrl ? null : <ImagePlus aria-hidden="true" />}
                </div>
                <span>
                  <strong>
                    {backgroundChoice === "replace" && backgroundPath ? fileNameFromPath(backgroundPath) : null}
                    {backgroundChoice === "keep" ? `保留 ${theme?.diy?.background_file_name || "当前主题背景"}` : null}
                    {backgroundChoice === "remove" ? "保存后移除背景" : null}
                    {backgroundChoice === "none" ? "自动中性背景" : null}
                  </strong>
                  <small className={backgroundChoice === "keep" && backgroundLoadError ? "is-error" : ""}>
                    {backgroundChoice === "keep" && backgroundLoadError
                      ? backgroundLoadError
                      : backgroundPreviewLoading
                        ? "正在安全读取并校验预览..."
                        : "PNG、JPEG 或 WebP，最大 8 MB"}
                  </small>
                </span>
                <Button className="codex-diy-background-select" type="button" variant="outline" size="sm" disabled={controlsDisabled} onClick={() => void chooseBackground()}>
                  <ImagePlus aria-hidden="true" />选择图片
                </Button>
                {backgroundChoice !== "none" ? (
                  <button type="button" className="codex-diy-background-remove" title="移除背景" aria-label="移除主题背景" disabled={controlsDisabled} onClick={removeBackground}>
                    <Trash2 aria-hidden="true" />
                  </button>
                ) : null}
                {hasExistingBackground && backgroundChoice !== "keep" ? (
                  <button type="button" className="codex-diy-background-keep" title="保留原背景" aria-label="保留当前主题背景" disabled={controlsDisabled} onClick={keepBackground}>
                    <RotateCcw aria-hidden="true" />
                  </button>
                ) : null}
              </div>
              <div className="codex-diy-segment-field codex-diy-image-layout-field">
                <span>图片显示方式</span>
                <div className="codex-diy-segmented codex-diy-image-layout" role="radiogroup" aria-label="图片显示方式">
                  <button type="button" role="radio" aria-checked={draft.settings.image_layout === "fullscreen"} className={draft.settings.image_layout === "fullscreen" ? "is-active" : ""} disabled={controlsDisabled} onClick={() => updateSetting("image_layout", "fullscreen")}>
                    <Maximize2 aria-hidden="true" />全屏透明背景
                  </button>
                  <button type="button" role="radio" aria-checked={draft.settings.image_layout === "banner"} className={draft.settings.image_layout === "banner" ? "is-active" : ""} disabled={controlsDisabled} onClick={() => updateSetting("image_layout", "banner")}>
                    <PanelTop aria-hidden="true" />上方长条
                  </button>
                  <button type="button" role="radio" aria-checked={draft.settings.image_layout === "card"} className={draft.settings.image_layout === "card" ? "is-active" : ""} disabled={controlsDisabled} onClick={() => updateSetting("image_layout", "card")}>
                    <LayoutDashboard aria-hidden="true" />中央大卡片
                  </button>
                </div>
              </div>
            </fieldset>
          </form>

          <section className="codex-diy-preview-pane" aria-labelledby="codex-diy-preview-title">
            <header>
              <div><strong id="codex-diy-preview-title">实时 Codex 预览</strong><span>控件修改即时呈现</span></div>
              <div><span>图片主色调</span><span>{IMAGE_LAYOUT_LABELS[draft.settings.image_layout]}</span></div>
            </header>
            <div className="codex-diy-preview-stage">
              <CodexDiyPreview draft={draft} backgroundUrl={backgroundUrl} />
            </div>
            <p><Sparkles aria-hidden="true" />{IMAGE_LAYOUT_DESCRIPTIONS[draft.settings.image_layout]}</p>
          </section>
        </div>

        <footer className="codex-diy-dialog-footer">
          <div aria-live="polite">
            {submitError ? <span className="is-error">{submitError}</span> : validationError ? <span className="is-error">{validationError}</span> : dirty ? <span>有未保存修改</span> : <span>设置已同步到预览</span>}
          </div>
          <div>
            <Button type="button" variant="outline" disabled={saving} onClick={requestClose}>取消</Button>
            <Button type="button" variant="outline" disabled={controlsDisabled || Boolean(validationError)} onClick={() => void save(false)}>
              {saving ? <LoaderCircle className="spin" aria-hidden="true" /> : null}
              保存到主题中心
            </Button>
            <Button type="button" disabled={controlsDisabled || Boolean(validationError)} onClick={() => void save(true)}>
              {saving ? <LoaderCircle className="spin" aria-hidden="true" /> : <Sparkles aria-hidden="true" />}
              保存并应用
            </Button>
          </div>
        </footer>
      </section>
    </div>,
    portalTarget,
  );
}
