import { useCallback, useEffect, useRef } from "react";
import { hideToast, type ToastVariant, useToast } from "./toast";

export const GLOW: Record<ToastVariant, string> = {
  success: "[--glow:var(--color-green)]",
  failure: "[--glow:var(--color-red)]",
  warning: "[--glow:var(--color-yellow)]",
  pending: "[--glow:transparent]",
};

const DOT: Record<Exclude<ToastVariant, "pending">, string> = {
  success: "bg-green shadow-[0_0_12px_8px_rgb(from_var(--color-green)_r_g_b/20%)]",
  failure: "bg-red shadow-[0_0_12px_8px_rgb(from_var(--color-red)_r_g_b/20%)]",
  warning: "bg-yellow shadow-[0_0_12px_8px_rgb(from_var(--color-yellow)_r_g_b/20%)]",
};

export function ActivityIndicator({ variant, className = "" }: { variant: ToastVariant; className?: string }) {
  if (variant === "pending") {
    return (
      <svg viewBox="0 0 16 16" aria-hidden="true" className={`size-4 shrink-0 animate-spin text-fg ${className}`}>
        <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeOpacity="0.2" strokeWidth="1.5" />
        <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeDasharray="9.4 28.3" strokeLinecap="round" />
      </svg>
    );
  }
  return (
    <span aria-hidden="true" className={`grid size-4 shrink-0 place-items-center ${className}`}>
      <span className={`size-[5px] rounded-full ${DOT[variant]}`} />
    </span>
  );
}

const FOOTER_CLOSE_MS = 3000;

export function FooterToast() {
  const toast = useToast();
  const ref = useRef<HTMLDivElement>(null);
  const cancel = useRef<() => void>(() => {});

  const schedule = useCallback(() => {
    cancel.current();
    if (!toast || toast.variant === "pending") return;
    let fade: Animation | undefined;
    const timer = window.setTimeout(() => {
      fade = ref.current?.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 200 });
      if (fade) fade.onfinish = hideToast;
      else hideToast();
    }, FOOTER_CLOSE_MS);
    cancel.current = () => {
      window.clearTimeout(timer);
      fade?.cancel();
    };
  }, [toast]);

  useEffect(() => {
    schedule();
    return () => cancel.current();
  }, [schedule]);

  if (!toast) return null;
  return (
    <>
      <div
        aria-hidden="true"
        className={`toast-glow pointer-events-none absolute top-[48px] left-[88px] -z-10 h-[400px] w-[10000px] -translate-x-1/2 -translate-y-1/2 rotate-[20deg] opacity-20 ${GLOW[toast.variant]}`}
      />
      <div
        ref={ref}
        role="status"
        onPointerEnter={() => cancel.current()}
        onPointerLeave={schedule}
        className="flex min-w-0 items-center gap-[7px] pl-0.5"
      >
        <ActivityIndicator variant={toast.variant} />
        <span className="truncate text-12 text-fg">{toast.title}</span>
        {toast.message && <span className="truncate text-fg-60">{toast.message}</span>}
      </div>
    </>
  );
}
