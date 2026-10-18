import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Toast } from "./toast";
import { ActivityIndicator, GLOW } from "./FooterToast";

const CLOSE_MS = 1200;
const FAILURE_CLOSE_MS = 3000;

export function Hud() {
  const [toast, setToast] = useState<(Toast & { shownAt: number }) | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<Toast>("hud-show", (event) => setToast({ ...event.payload, shownAt: Date.now() }));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  useLayoutEffect(() => {
    const rect = ref.current?.getBoundingClientRect();
    if (!toast || !rect) return;
    invoke("hud_present", { width: Math.ceil(rect.width), height: Math.ceil(rect.height) });
    if (toast.variant === "pending") return;
    const timer = window.setTimeout(
      () => {
        invoke("hud_hide");
        setToast(null);
      },
      toast.variant === "failure" ? FAILURE_CLOSE_MS : CLOSE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [toast]);

  if (!toast) return null;
  return (
    <div
      ref={ref}
      role="status"
      className={`relative inline-flex max-w-[500px] items-center gap-3 overflow-hidden bg-bg pr-6 text-13 ${toast.message ? "h-14" : "h-[46px]"} ${toast.icon ? "pl-4" : "pl-6"}`}
    >
      <div aria-hidden="true" className={`hud-glow absolute inset-0 ${GLOW[toast.variant]}`} />
      {toast.icon && (
        <div className="relative flex size-[22px] shrink-0 items-center justify-center">
          <img
            src={toast.icon}
            alt=""
            draggable={false}
            className="size-[22px] [mask-image:radial-gradient(circle_6px_at_20px_20px,transparent_85%,black_100%)]"
          />
          <ActivityIndicator variant={toast.variant} className="absolute -right-1.5 -bottom-1.5" />
        </div>
      )}
      <div className={`relative flex min-w-0 flex-col justify-center gap-0.5 whitespace-nowrap ${toast.message ? "-top-px" : "top-px"}`}>
        <h1 className="truncate font-medium text-fg">{toast.title}</h1>
        {toast.message && <p className="truncate text-11 text-fg-60">{toast.message}</p>}
      </div>
    </div>
  );
}
