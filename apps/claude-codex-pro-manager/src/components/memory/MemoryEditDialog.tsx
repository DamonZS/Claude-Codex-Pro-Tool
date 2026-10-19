import { useEffect, useState } from "react";
import { Download, X } from "lucide-react";

import type { MemoryKind, MemoryLibraryEntry, MemorySaveInput } from "./types";

import "./memory-library.css";

const KIND_OPTIONS: ReadonlyArray<{ value: MemoryKind; label: string; hint: string }> = [
  { value: "persona", label: "画像", hint: "关于你这个人：偏好、习惯、口味" },
  { value: "memory", label: "任务记忆", hint: "关于约定与决策：我们定了什么规矩" },
];

export function MemoryEditDialog({
  entry,
  busy = false,
  onClose,
  onExport,
  onSave,
}: {
  entry: MemoryLibraryEntry | null;
  busy?: boolean;
  onClose: () => void;
  onExport: (entry: MemoryLibraryEntry) => void;
  onSave: (input: MemorySaveInput) => void;
}) {
  const [kind, setKind] = useState<MemoryKind>(entry?.kind ?? "memory");
  const [title, setTitle] = useState(entry?.title ?? "");
  const [body, setBody] = useState(entry?.body ?? "");

  useEffect(() => {
    setKind(entry?.kind ?? "memory");
    setTitle(entry?.title ?? "");
    setBody(entry?.body ?? "");
  }, [entry]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, onClose]);

  const canSave = title.trim().length > 0 && body.trim().length > 0;
  const close = () => {
    if (!busy) onClose();
  };
  const save = () => {
    if (busy || !canSave) return;
    onSave({ id: entry?.id ?? null, kind, title: title.trim(), body: body.trim() });
  };

  return (
    <div className="ml-dialog-layer" onClick={close} role="presentation">
      <div
        aria-label={entry ? "编辑记忆" : "新增记忆"}
        aria-modal="true"
        className="ml-dialog"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="ml-dialog-head">
          <h2>{entry ? "编辑记忆" : "新增记忆"}</h2>
          <button aria-label="关闭" className="ml-dialog-close" disabled={busy} onClick={close} type="button">
            <X aria-hidden="true" />
          </button>
        </header>

        <form
          className="ml-dialog-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <div className="ml-dialog-field">
            <span className="ml-dialog-label">是什么</span>
            <div aria-label="记忆类型" className="ml-kind-options" role="group">
              {KIND_OPTIONS.map((option) => (
                <button
                  aria-pressed={kind === option.value}
                  className={kind === option.value ? "ml-kind-option on" : "ml-kind-option"}
                  disabled={busy}
                  key={option.value}
                  onClick={() => setKind(option.value)}
                  type="button"
                >
                  <strong>{option.label}</strong>
                  <small>{option.hint}</small>
                </button>
              ))}
            </div>
          </div>

          <label className="ml-dialog-field">
            <span className="ml-dialog-label">一句话标题</span>
            <input
              maxLength={256}
              onChange={(event) => setTitle(event.currentTarget.value)}
              placeholder="例如：CPP 任务记忆 · 6 sessions"
              value={title}
            />
          </label>

          <label className="ml-dialog-field">
            <span className="ml-dialog-label">说明</span>
            <textarea
              maxLength={24000}
              onChange={(event) => setBody(event.currentTarget.value)}
              placeholder="完整记忆内容，支持 Markdown"
              rows={9}
              value={body}
            />
          </label>

          <footer className="ml-dialog-actions">
            <button className="ml-dialog-button" disabled={busy} onClick={close} type="button">取消</button>
            {entry ? (
              <button className="ml-dialog-button" disabled={busy} onClick={() => onExport(entry)} type="button">
                <Download aria-hidden="true" />导出 MD
              </button>
            ) : null}
            <button className="ml-dialog-button primary" disabled={busy || !canSave} type="submit">保存</button>
          </footer>
        </form>
      </div>
    </div>
  );
}
