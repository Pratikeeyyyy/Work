import { useEffect, useRef, type ReactNode } from "react";
import { Icon } from "./Icon";

export default function Modal({
  open,
  title,
  onClose,
  children,
  footer,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}) {
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    panel.current?.focus();
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-40 overflow-y-auto">
      <div className="fixed inset-0 bg-slate-900/50" onClick={onClose} aria-hidden="true" />
      <div className="relative z-10 flex min-h-full items-end justify-center p-4 sm:items-center">
        <div
          ref={panel}
          role="dialog"
          aria-modal="true"
          aria-label={title}
          tabIndex={-1}
          className="w-full max-w-lg rounded-xl bg-white shadow-xl outline-none"
        >
          <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
            <h2 className="text-base font-semibold text-slate-900">{title}</h2>
            <button
              onClick={onClose}
              className="rounded-md p-1 text-slate-400 transition-colors hover:bg-slate-100 hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500"
              aria-label="Close dialog"
            >
              <Icon name="close" className="h-5 w-5" />
            </button>
          </div>
          <div className="max-h-[70vh] overflow-y-auto px-5 py-4">{children}</div>
          {footer && (
            <div className="flex justify-end gap-2 border-t border-slate-200 px-5 py-3">
              {footer}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}