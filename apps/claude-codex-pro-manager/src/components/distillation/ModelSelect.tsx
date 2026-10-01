import { useEffect, useMemo, useRef, useState } from "react";
import { Check, ChevronDown, Search } from "lucide-react";

export interface DistillModelOption {
  /** `${providerId}::${model}` or "offline". */
  readonly id: string;
  readonly providerId: string;
  readonly model: string;
  readonly label: string;
  readonly sub?: string;
  readonly vendor?: string;
  readonly offline?: boolean;
  readonly ok?: boolean;
  /** The supplier/model CCP is currently running. */
  readonly active?: boolean;
}

/** Searchable, vendor-grouped model picker adapted from AITracker ModelSelect. */
export function ModelSelect({
  options,
  value,
  onChange,
  onManage,
}: {
  options: readonly DistillModelOption[];
  value: string;
  onChange: (value: string) => void;
  onManage: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);
  const current = options.find((option) => option.id === value) ?? options[0];

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const groups = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase();
    const list = keyword
      ? options.filter((option) => `${option.label} ${option.sub ?? ""} ${option.vendor ?? ""}`.toLocaleLowerCase().includes(keyword))
      : options;
    const map = new Map<string, DistillModelOption[]>();
    for (const option of list) {
      const vendor = option.vendor ?? "自有";
      map.set(vendor, [...(map.get(vendor) ?? []), option]);
    }
    return [...map.entries()];
  }, [options, query]);

  return (
    <div className="dw-model-select" ref={rootRef}>
      <button aria-expanded={open} aria-haspopup="listbox" className="dw-model-trigger" onClick={() => setOpen((state) => !state)} type="button">
        <span className={`dw-status-dot${current?.ok ? " ok" : ""}`} />
        <span className="dw-model-name">{current?.offline ? "离线回退（确定性）" : current?.label ?? "未配置模型"}</span>
        <span className="dw-model-sub">{current?.offline ? "" : current?.sub}</span>
        <ChevronDown aria-hidden="true" className={open ? "rotated" : ""} />
      </button>
      {open ? (
        <div className="dw-model-menu" role="listbox">
          <label className="dw-search">
            <Search aria-hidden="true" />
            <input autoFocus onChange={(event) => setQuery(event.target.value)} placeholder="搜索模型…" value={query} />
          </label>
          <div className="dw-model-scroll">
            {groups.length === 0 ? <div className="dw-model-empty">没有匹配模型</div> : null}
            {groups.map(([vendor, items]) => (
              <div className="dw-model-group" key={vendor}>
                <div className="dw-model-vendor">{vendor}</div>
                {items.map((option) => {
                  const on = option.id === value;
                  return (
                    <button
                      aria-selected={on}
                      className={`dw-model-option${on ? " active" : ""}`}
                      key={option.id}
                      onClick={() => { onChange(option.id); setOpen(false); setQuery(""); }}
                      role="option"
                      type="button"
                    >
                      <span className={`dw-status-dot${option.ok ? " ok" : ""}`} />
                      <span className="dw-model-option-copy">
                        <strong>{option.offline ? "离线回退（确定性）" : option.label}</strong>
                        <small>{option.sub}{option.active ? " · 当前使用" : ""}</small>
                      </span>
                      {on ? <Check aria-hidden="true" /> : null}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
          <button className="dw-model-add" onClick={() => { setOpen(false); onManage(); }} type="button">+ 新增自有模型配置</button>
        </div>
      ) : null}
    </div>
  );
}
