import { Check } from "lucide-react";
import type { ReactNode } from "react";

export function Switch({ checked, onChange, labelledBy }: { checked: boolean; onChange: (checked: boolean) => void; labelledBy: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      onClick={() => onChange(!checked)}
      className={`relative h-4 w-7 shrink-0 rounded-full outline-offset-2 transition-colors ${checked ? "bg-switch-on" : "border border-switch-off"}`}
    >
      <span
        className={`absolute top-1/2 -translate-y-1/2 rounded-full transition-all ${checked ? "right-[3px] size-2.5 bg-white" : "left-[3px] size-2 bg-fg opacity-90"}`}
      />
    </button>
  );
}

export function Checkbox({ checked, onChange, label }: { checked: boolean; onChange: (checked: boolean) => void; label: string }) {
  return (
    <span className="relative flex size-4 shrink-0">
      <input
        type="checkbox"
        aria-label={label}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="peer size-4 appearance-none rounded-[4px] border border-switch-off checked:border-transparent checked:bg-control-hover"
      />
      <Check aria-hidden="true" strokeWidth={3} className="pointer-events-none absolute inset-0.5 hidden size-3 text-fg peer-checked:block" />
    </span>
  );
}

export function Row({ id, title, description, children }: { id: string; title: string; description?: string; children: ReactNode }) {
  return (
    <div className="flex min-h-[47px] items-center justify-between gap-3 border-b border-card-separator px-3 py-2 last:min-h-[46px] last:border-b-0">
      <div className="flex min-w-0 flex-col gap-1">
        <span id={id} className="font-medium">
          {title}
        </span>
        {description && <span className="text-12 text-fg-60">{description}</span>}
      </div>
      <div className="flex shrink-0 items-center gap-2">{children}</div>
    </div>
  );
}

export function Card({ children }: { children: ReactNode }) {
  return <section className="flex flex-col overflow-hidden rounded-lg bg-card">{children}</section>;
}
