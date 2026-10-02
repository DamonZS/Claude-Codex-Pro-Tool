import { useState } from "react";
import type { LucideIcon } from "lucide-react";
import { ChevronDown } from "lucide-react";

/** Collapsible settings card (CC Switch accordion item). */
export function SettingsSection({
  icon: Icon,
  tone,
  title,
  description,
  badge,
  defaultOpen = false,
  children,
}: {
  icon: LucideIcon;
  tone: string;
  title: string;
  description: string;
  badge?: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <section className={`st-section${open ? " open" : ""}`}>
      <button aria-expanded={open} className="st-section-head" onClick={() => setOpen((value) => !value)} type="button">
        <span className="st-section-icon" style={{ color: tone }}><Icon aria-hidden="true" /></span>
        <span className="st-section-copy">
          <strong>{title}{badge}</strong>
          <small>{description}</small>
        </span>
        <ChevronDown aria-hidden="true" className="st-section-chevron" />
      </button>
      {open ? <div className="st-section-body">{children}</div> : null}
    </section>
  );
}

/** Icon + title + description row with a trailing switch. */
export function ToggleRow({
  icon: Icon,
  tone,
  title,
  description,
  checked,
  disabled,
  onChange,
}: {
  icon?: LucideIcon;
  tone?: string;
  title: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className={`st-toggle-row${disabled ? " disabled" : ""}`}>
      {Icon ? <span className="st-row-icon" style={{ color: tone }}><Icon aria-hidden="true" /></span> : null}
      <span className="st-row-copy">
        <strong>{title}</strong>
        {description ? <small>{description}</small> : null}
      </span>
      <Switch checked={checked} disabled={disabled} label={title} onChange={onChange} />
    </div>
  );
}

export function Switch({ checked, disabled, label, onChange }: { checked: boolean; disabled?: boolean; label: string; onChange: (value: boolean) => void }) {
  return (
    <button
      aria-checked={checked}
      aria-label={label}
      className={`st-switch${checked ? " on" : ""}`}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      role="switch"
      type="button"
    >
      <span />
    </button>
  );
}

/** Segmented single-choice control. */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: ReadonlyArray<{ value: T; label: string; icon?: LucideIcon }>;
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div aria-label={label} className="st-segmented" role="radiogroup">
      {options.map((option) => {
        const Icon = option.icon;
        return (
          <button aria-checked={value === option.value} className={value === option.value ? "on" : ""} key={option.value} onClick={() => onChange(option.value)} role="radio" type="button">
            {Icon ? <Icon aria-hidden="true" /> : null}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

/** Uncollapsed group heading + hint + control (General tab style). */
export function SettingsField({ title, hint, children, footnote }: { title: string; hint?: string; children: React.ReactNode; footnote?: string }) {
  return (
    <div className="st-field">
      <strong>{title}</strong>
      {hint ? <small>{hint}</small> : null}
      <div className="st-field-control">{children}</div>
      {footnote ? <p className="st-footnote">{footnote}</p> : null}
    </div>
  );
}

export function NumberField({
  label,
  hint,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  hint?: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  const invalid = !Number.isFinite(value) || value < min || value > max;
  return (
    <label className={`st-number${invalid ? " invalid" : ""}`}>
      <span>{label}</span>
      <input max={max} min={min} onChange={(event) => onChange(Number(event.target.value))} type="number" value={Number.isFinite(value) ? value : ""} />
      {hint ? <small>{hint}</small> : null}
    </label>
  );
}
