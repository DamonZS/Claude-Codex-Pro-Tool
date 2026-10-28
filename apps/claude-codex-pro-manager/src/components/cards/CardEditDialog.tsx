import { useEffect, useState } from "react";
import { Download, X } from "lucide-react";

import type { KnowledgeCard } from "@/types";

import {
  CARD_KIND_OPTIONS,
  cardDraftFromCard,
  cardDraftToSaveInput,
  type CardDraft,
} from "./types";

import "./card-library.css";

/**
 * Optional edit dialog. Opening it is never required: cards can be created with
 * the inline quick-create form and searched / exported straight from the list.
 */
export function CardEditDialog({
  card,
  busy = false,
  onClose,
  onExport,
  onSave,
}: {
  card: KnowledgeCard;
  busy?: boolean;
  onClose: () => void;
  onExport: (card: KnowledgeCard) => void;
  onSave: (input: ReturnType<typeof cardDraftToSaveInput>) => void;
}) {
  const [draft, setDraft] = useState<CardDraft>(() => cardDraftFromCard(card));

  useEffect(() => {
    setDraft(cardDraftFromCard(card));
  }, [card]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, onClose]);

  const canSave = draft.body.trim().length > 0;
  const close = () => {
    if (!busy) onClose();
  };
  const save = () => {
    if (busy || !canSave) return;
    onSave(cardDraftToSaveInput(draft, card.id));
  };

  return (
    <div className="cl-dialog-layer" onClick={close} role="presentation">
      <div
        aria-label="编辑卡片"
        aria-modal="true"
        className="cl-dialog"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="cl-dialog-head">
          <h2>编辑卡片</h2>
          <button aria-label="关闭" className="cl-dialog-close" disabled={busy} onClick={close} type="button">
            <X aria-hidden="true" />
          </button>
        </header>

        <form
          className="cl-dialog-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <div className="cl-dialog-field">
            <span className="cl-dialog-label">卡片类型</span>
            <div aria-label="卡片类型" className="cl-kind-options" role="group">
              {CARD_KIND_OPTIONS.map((option) => (
                <button
                  aria-pressed={draft.kind === option.value}
                  className={draft.kind === option.value ? "cl-kind-option on" : "cl-kind-option"}
                  disabled={busy}
                  key={option.value}
                  onClick={() => setDraft((current) => ({ ...current, kind: option.value }))}
                  type="button"
                >
                  <strong>{option.label}</strong>
                  <small>{option.hint}</small>
                </button>
              ))}
            </div>
          </div>

          <label className="cl-dialog-field">
            <span className="cl-dialog-label">标题（留空则由正文首行生成）</span>
            <input
              maxLength={256}
              onChange={(event) => setDraft((current) => ({ ...current, title: event.currentTarget.value }))}
              placeholder="例如：SSE 断流重连要复用同一条连接"
              value={draft.title}
            />
          </label>

          <label className="cl-dialog-field">
            <span className="cl-dialog-label">正文</span>
            <textarea
              className="is-tall"
              onChange={(event) => setDraft((current) => ({ ...current, body: event.currentTarget.value }))}
              placeholder="卡片正文，支持 Markdown；这段内容会同时用于导出与 Skill 生成"
              rows={9}
              value={draft.body}
            />
          </label>

          <label className="cl-dialog-field">
            <span className="cl-dialog-label">标签（逗号分隔）</span>
            <input
              onChange={(event) => setDraft((current) => ({ ...current, tags: event.currentTarget.value }))}
              placeholder="例如：codex，发布，验证"
              value={draft.tags}
            />
          </label>

          <footer className="cl-dialog-actions">
            <button className="cl-dialog-button" disabled={busy} onClick={close} type="button">取消</button>
            <button className="cl-dialog-button" disabled={busy} onClick={() => onExport(card)} type="button">
              <Download aria-hidden="true" />导出单卡
            </button>
            <button className="cl-dialog-button primary" disabled={busy || !canSave} type="submit">保存修改</button>
          </footer>
        </form>
      </div>
    </div>
  );
}