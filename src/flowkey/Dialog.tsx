import { useEffect, useRef } from "react";
import { useDialog } from "./dialogs";

export function FlowKeyDialog() {
  const request = useDialog();
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (request) cancelRef.current?.focus();
  }, [request]);
  if (!request) return null;
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/40" onMouseDown={(e) => e.target === e.currentTarget && request.resolve(false)}>
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="flowkey-dialog-title"
        aria-describedby={request.message ? "flowkey-dialog-message" : undefined}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Escape") {
            e.preventDefault();
            request.resolve(false);
          } else if (e.key === "Tab") {
            e.preventDefault();
            const buttons = [...e.currentTarget.querySelectorAll("button")];
            buttons[(buttons.indexOf(document.activeElement as HTMLButtonElement) + 1) % buttons.length]?.focus();
          }
        }}
        className="flex w-[320px] flex-col items-center gap-4 rounded-panel border border-panel-border bg-panel p-4 text-center text-13 shadow-panel"
      >
        <div className="flex w-full flex-col gap-1">
          <h2 id="flowkey-dialog-title" className="font-semibold text-fg">
            {request.title}
          </h2>
          {request.message && (
            <p id="flowkey-dialog-message" className="text-fg-60">
              {request.message}
            </p>
          )}
          {request.items && request.items.length > 0 && (
            <ul className="mt-2 max-h-[150px] overflow-y-auto rounded-[6px] bg-control px-3 py-2 text-left text-12 text-fg">
              {request.items.map((item) => (
                <li key={item} className="truncate py-0.5" title={item}>
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="flex w-full flex-col gap-2">
          <button
            type="button"
            onClick={() => request.resolve(true)}
            className={`h-8 rounded-[6px] bg-control font-medium outline-offset-2 hover:bg-control-hover focus-visible:outline-2 focus-visible:outline-accent ${request.destructive ? "text-destructive" : "text-fg"}`}
          >
            {request.confirmTitle}
          </button>
          <button
            ref={cancelRef}
            type="button"
            onClick={() => request.resolve(false)}
            className="h-8 rounded-[6px] bg-control font-medium text-fg outline-offset-2 hover:bg-control-hover focus-visible:outline-2 focus-visible:outline-accent"
          >
            {request.cancelTitle}
          </button>
        </div>
      </div>
    </div>
  );
}
