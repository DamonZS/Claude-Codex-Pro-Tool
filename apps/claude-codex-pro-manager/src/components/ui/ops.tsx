import { AlertTriangle, CheckCircle2, X, type LucideIcon } from "lucide-react";
import type React from "react";

import { statusOk } from "@/lib/helpers";
import type { Status, StatusChip } from "@/types";

export function Panel({ title, detail, hideHeader = false, children }: { title: string; detail?: string; hideHeader?: boolean; children: React.ReactNode }) {
  return (
    <section className="ops-panel">
      {hideHeader ? null : (
        <header>
          <div>
            <h2>{title}</h2>
            {detail ? <p>{detail}</p> : null}
          </div>
        </header>
      )}
      <div className="ops-panel-body">{children}</div>
    </section>
  );
}

export function StatusTile({ icon: Icon, label, value, status, items }: { icon: LucideIcon; label: string; value?: string; status: string; items?: StatusChip[] }) {
  return (
    <div className={`status-tile ${statusOk(status) ? "ok" : "warn"}`}>
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      {items?.length ? (
        <div className="status-segment-list">
          {items.map((item, index) => (
            <b className={`status-segment ${item.tone}`} key={index}>{item.label}</b>
          ))}
        </div>
      ) : (
        <strong>{value}</strong>
      )}
    </div>
  );
}

export function StatusActionTile({ disabled, icon: Icon, label, value, status, onClick }: { disabled?: boolean; icon: LucideIcon; label: string; value: string; status: string; onClick: () => void }) {
  return (
    <button className={`status-tile status-action-tile ${statusOk(status) ? "ok" : "warn"}`} disabled={disabled} onClick={onClick} type="button">
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      <div className="status-segment-list">
        <b className={`status-segment ${statusOk(status) ? "ok" : "muted"}`}>{value}</b>
      </div>
    </button>
  );
}

export function ActionButton({ icon: Icon, label, onClick }: { icon: LucideIcon; label: string; onClick: () => void }) {
  return (
    <button className="action-button" onClick={onClick} type="button">
      <Icon className="h-4 w-4" />
      <span>{label}</span>
    </button>
  );
}

export function ToggleSwitch({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <button
      aria-pressed={checked}
      className={`toggle-switch ${checked ? "checked" : ""}`}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      type="button"
    >
      <span className="toggle-switch-thumb" />
    </button>
  );
}

export function InfoRow({ action, label, value }: { action?: React.ReactNode; label: string; value: string }) {
  return (
    <div className={action ? "info-row with-action" : "info-row"}>
      <span>{label}</span>
      <strong>{value}</strong>
      {action ? <div className="info-row-action-wrap">{action}</div> : null}
    </div>
  );
}

export function StatusRow({ label, value, status }: { label: string; value: string; status: string }) {
  return (
    <div className={`ops-status-row ${statusOk(status) ? "ok" : "warn"}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function Empty({ text }: { text: string }) {
  return <div className="empty-state">{text}</div>;
}

export function Notice({ notice, onClose }: { notice: { title: string; message: string; status?: Status }; onClose: () => void }) {
  const ok = statusOk(notice.status);
  const running = notice.status === "running";
  return (
    <div className="toast-wrap" role="status" aria-live={ok ? "polite" : "assertive"}>
      <div className={`${ok ? "toast-card" : "toast-card failed"}${running ? " running" : ""}`}>
        <div className="toast-progress" />
        <div className="toast-icon">{ok ? <CheckCircle2 className="h-5 w-5" /> : <AlertTriangle className="h-5 w-5" />}</div>
        <div className="toast-body">
          <h2>{notice.title}</h2>
          <p>{notice.message}</p>
        </div>
        <button className="toast-close" onClick={onClose} type="button" aria-label="关闭提示">
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
