import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "./utils/cn";
import { CheckIcon, ChevronDown, CloseIcon } from "./icons";

/* =========================================================================
 * Buttons
 * ========================================================================= */
type BtnVariant = "filled" | "tonal" | "outlined" | "text" | "plain" | "danger" | "destructive";
type BtnSize = "sm" | "md" | "lg";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: BtnVariant;
  size?: BtnSize;
  icon?: ReactNode;
  loading?: boolean;
};

export function Button({
  variant = "filled",
  size = "md",
  icon,
  loading,
  className,
  children,
  disabled,
  ...rest
}: ButtonProps) {
  const sizes: Record<BtnSize, string> = {
    sm: "h-8 px-3.5 text-[13px] gap-1.5 rounded-full",
    md: "h-10 px-5 text-[14px] gap-2 rounded-full",
    lg: "h-12 px-6 text-[15px] gap-2 rounded-full",
  };
  const variants: Record<BtnVariant, string> = {
    filled:
      "bg-[var(--color-accent)] text-white shadow-sm shadow-[var(--color-accent)]/25 hover:bg-[var(--color-accent-hover)] active:brightness-95",
    tonal:
      "bg-[var(--color-accent-soft)] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/20 active:bg-[var(--color-accent)]/25",
    outlined:
      "bg-transparent text-[var(--color-on-surface)] ring-1 ring-inset ring-[var(--color-outline)] hover:bg-black/[0.04] dark:hover:bg-white/[0.06]",
    text: "bg-transparent text-[var(--color-on-surface)] hover:bg-black/[0.04] dark:hover:bg-white/[0.06]",
    plain: "bg-transparent text-[var(--color-accent)] hover:bg-[var(--color-accent-soft)]",
    danger:
      "bg-[var(--color-destructive-soft)] text-[var(--color-destructive)] hover:bg-[var(--color-destructive)]/25 ring-1 ring-inset ring-[var(--color-destructive)]/25",
    destructive:
      "bg-[var(--color-destructive)] text-white hover:bg-[var(--color-destructive-hover)] active:brightness-95",
  };
  return (
    <button
      {...rest}
      disabled={disabled || loading}
      className={cn(
        "inline-flex items-center justify-center font-medium transition-all duration-150 select-none",
        "disabled:opacity-40 disabled:cursor-not-allowed",
        sizes[size],
        variants[variant],
        className
      )}
    >
      {loading ? <Spinner size={16} className="text-current" /> : icon}
      {children}
    </button>
  );
}

export function IconButton({
  className,
  children,
  active,
  size = "md",
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean; size?: "sm" | "md" | "lg" }) {
  const sizes = { sm: "size-8", md: "size-10", lg: "size-12" };
  return (
    <button
      {...rest}
      className={cn(
        "inline-flex items-center justify-center rounded-full transition-all duration-150",
        "text-[var(--color-on-surface-muted)] hover:bg-black/[0.05] hover:text-[var(--color-on-surface)] active:scale-95",
        "dark:hover:bg-white/[0.08]",
        active && "bg-[var(--color-accent-soft)] text-[var(--color-accent)] hover:bg-[var(--color-accent-soft)] hover:text-[var(--color-accent)]",
        sizes[size],
        className
      )}
    >
      {children}
    </button>
  );
}

/* =========================================================================
 * Chip / Badge / Segmented
 * ========================================================================= */
export function Chip({
  children,
  selected,
  onClick,
  icon,
  className,
}: {
  children: ReactNode;
  selected?: boolean;
  onClick?: () => void;
  icon?: ReactNode;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1.5 h-8 px-3 text-[13px] rounded-full transition-all duration-150 ring-1 ring-inset active:scale-[0.97]",
        selected
          ? "bg-[var(--color-accent-soft)] text-[var(--color-accent)] ring-[var(--color-accent)]/30"
          : "bg-transparent text-[var(--color-on-surface-muted)] ring-[var(--color-outline)] hover:bg-black/[0.04] hover:text-[var(--color-on-surface)] dark:hover:bg-white/[0.05]",
        className
      )}
    >
      {selected && <CheckIcon className="size-3.5" />}
      {!selected && icon}
      {children}
    </button>
  );
}

export function Badge({
  children,
  tone = "neutral",
  className,
}: {
  children: ReactNode;
  tone?: "neutral" | "primary" | "accent" | "success" | "warning" | "danger";
  className?: string;
}) {
  const tones = {
    neutral: "bg-[var(--color-surface-3)] text-[var(--color-on-surface-muted)] ring-[var(--color-outline-soft)]",
    primary: "bg-[var(--color-accent-soft)] text-[var(--color-accent)] ring-[var(--color-accent)]/25",
    accent: "bg-[var(--color-accent-soft)] text-[var(--color-accent)] ring-[var(--color-accent)]/25",
    success: "bg-[var(--color-success)]/15 text-[var(--color-success)] ring-[var(--color-success)]/25",
    warning: "bg-[var(--color-warning)]/15 text-[var(--color-warning)] ring-[var(--color-warning)]/25",
    danger: "bg-[var(--color-destructive-soft)] text-[var(--color-destructive)] ring-[var(--color-destructive)]/25",
  };
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 px-2 h-6 rounded-full text-[11px] font-medium tracking-wide ring-1 ring-inset",
        tones[tone],
        className
      )}
    >
      {children}
    </span>
  );
}

export function Segmented<T extends string>({
  value,
  onChange,
  options,
  className,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: ReactNode; icon?: ReactNode }[];
  className?: string;
}) {
  return (
    <div
      className={cn(
        "inline-flex p-0.5 rounded-full bg-[var(--color-surface-3)] ring-1 ring-inset ring-[var(--color-outline-soft)]",
        className
      )}
    >
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            onClick={() => onChange(o.value)}
            className={cn(
              "relative inline-flex items-center gap-1.5 h-8 px-3.5 text-[13px] font-medium rounded-full transition-colors duration-200",
              active
                ? "bg-[var(--color-surface-elevated)] text-[var(--color-on-surface)] shadow-sm"
                : "text-[var(--color-on-surface-muted)] hover:text-[var(--color-on-surface)]"
            )}
          >
            {o.icon}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/* =========================================================================
 * Progress / Switch
 * ========================================================================= */
export function Progress({
  value,
  className,
  tone = "primary",
}: {
  value: number;
  className?: string;
  tone?: "primary" | "accent" | "success";
}) {
  const tones = {
    primary: "bg-[var(--color-accent)]",
    accent: "bg-[var(--color-accent)]",
    success: "bg-[var(--color-success)]",
  };
  return (
    <div className={cn("h-1 w-full overflow-hidden rounded-full bg-black/10 dark:bg-white/10", className)}>
      <div
        className={cn("h-full rounded-full transition-all", tones[tone])}
        style={{ width: `${Math.min(100, Math.max(0, value * 100))}%` }}
      />
    </div>
  );
}

export function Switch({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative h-[31px] w-[51px] rounded-full transition-colors duration-200 disabled:opacity-40",
        checked ? "bg-[var(--color-success)]" : "bg-black/15 dark:bg-white/20"
      )}
    >
      <span
        className={cn(
          "absolute top-1/2 size-[27px] -translate-y-1/2 rounded-full bg-white shadow-[0_2px_4px_rgba(0,0,0,0.25)] transition-all duration-200",
          checked ? "left-[22px]" : "left-[2px]"
        )}
      />
    </button>
  );
}

/* =========================================================================
 * Snackbar
 * ========================================================================= */
export type SnackMsg = { id: number; text: string; tone?: "neutral" | "success" | "danger" };
export function Snackbar({ msg, onDismiss }: { msg: SnackMsg | null; onDismiss: () => void }) {
  useEffect(() => {
    if (!msg) return;
    const t = setTimeout(onDismiss, 3200);
    return () => clearTimeout(t);
  }, [msg, onDismiss]);
  if (!msg) return null;
  const tones = {
    neutral: "bg-[rgba(40,40,42,0.92)] text-white",
    success: "bg-[rgba(40,40,42,0.92)] text-white",
    danger: "bg-[var(--color-destructive)] text-white",
  };
  const dot = { neutral: "bg-white/50", success: "bg-[var(--color-success)]", danger: "bg-white" };
  return (
    <div
      className={cn(
        "anim-snackbar fixed bottom-8 left-1/2 z-[80] flex items-center gap-2.5 px-4 py-3 rounded-2xl shadow-xl ring-1 ring-white/10 text-[13px] font-medium backdrop-blur-xl",
        tones[msg.tone ?? "neutral"]
      )}
    >
      <span className={cn("size-1.5 rounded-full", dot[msg.tone ?? "neutral"])} />
      <span>{msg.text}</span>
      <button onClick={onDismiss} className="ml-1 opacity-60 hover:opacity-100 transition-opacity">
        <CloseIcon className="size-4" />
      </button>
    </div>
  );
}

export function useSnackbar() {
  const [msg, setMsg] = useState<SnackMsg | null>(null);
  const show = useCallback((text: string, tone?: SnackMsg["tone"]) => {
    setMsg({ id: Date.now(), text, tone });
  }, []);
  const dismiss = useCallback(() => setMsg(null), []);
  return { msg, show, dismiss };
}

/* =========================================================================
 * SearchField
 * ========================================================================= */
export function SearchField({
  value,
  onChange,
  placeholder,
  className,
  autoFocus,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
}) {
  return (
    <div
      className={cn(
        "inline-flex items-center gap-2 h-10 px-3.5 rounded-full bg-[var(--color-surface-2)] ring-1 ring-inset ring-[var(--color-outline-soft)] focus-within:ring-[var(--color-accent)]/40 transition-all",
        className
      )}
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} className="size-4 text-[var(--color-on-surface-faint)]">
        <circle cx="11" cy="11" r="7" />
        <path d="M20 20l-3.5-3.5" />
      </svg>
      <input
        value={value}
        autoFocus={autoFocus}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="bg-transparent outline-none text-sm text-[var(--color-on-surface)] placeholder:text-[var(--color-on-surface-faint)] flex-1 min-w-0"
      />
      {value && (
        <button onClick={() => onChange("")} className="text-[var(--color-on-surface-faint)] hover:text-[var(--color-on-surface)]">
          <CloseIcon className="size-4" />
        </button>
      )}
    </div>
  );
}

/* =========================================================================
 * Skeleton / Spinner / Card
 * ========================================================================= */
export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("skeleton rounded-lg", className)} />;
}

export function Spinner({ size = 18, className, stroke = 2 }: { size?: number; className?: string; stroke?: number }) {
  return (
    <span
      className={cn("nx-spinner inline-block text-[var(--color-on-surface-faint)]", className)}
      style={{ width: size, height: size, borderWidth: stroke }}
    />
  );
}

export function Card({
  children,
  className,
  elevated,
}: {
  children: ReactNode;
  className?: string;
  elevated?: boolean;
}) {
  return (
    <div
      className={cn(
        "rounded-[var(--radius-panel)] bg-[var(--color-surface-1)] ring-1 ring-inset ring-[var(--color-outline-soft)]",
        elevated && "shadow-lg shadow-black/[0.06]",
        className
      )}
    >
      {children}
    </div>
  );
}

/* =========================================================================
 * Floating layer primitive — shared by Popover / Menu / Dropdown / ContextMenu
 * ========================================================================= */
type Placement = "bottom-start" | "bottom-end" | "bottom" | "top-start" | "top-end" | "top";

function useAnchoredPosition(
  anchorRect: DOMRect | null,
  placement: Placement,
  offset = 6
) {
  const [pos, setPos] = useState<{ left: number; top: number; origin: string } | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!anchorRect) return;
    const panel = panelRef.current;
    const pw = panel?.offsetWidth ?? 220;
    const ph = panel?.offsetHeight ?? 200;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let top = anchorRect.bottom + offset;
    let origin = "top center";
    const wantsTop = placement.startsWith("top");
    if (wantsTop || top + ph > vh - 8) {
      top = anchorRect.top - ph - offset;
      origin = "bottom center";
    }
    top = Math.max(8, Math.min(top, vh - ph - 8));

    let left: number;
    if (placement.endsWith("end")) left = anchorRect.right - pw;
    else if (placement.endsWith("start")) left = anchorRect.left;
    else left = anchorRect.left + anchorRect.width / 2 - pw / 2;
    left = Math.max(8, Math.min(left, vw - pw - 8));

    setPos({ left, top, origin });
  }, [anchorRect, placement, offset]);

  return { pos, panelRef };
}

export function FloatingPanel({
  anchorRect,
  placement = "bottom-start",
  onClose,
  children,
  className,
  matchWidth,
  offset,
}: {
  anchorRect: DOMRect | null;
  placement?: Placement;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  matchWidth?: boolean;
  offset?: number;
}) {
  const { pos, panelRef } = useAnchoredPosition(anchorRect, placement, offset);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onPointer = (e: PointerEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("keydown", onKey);
    // pointerdown on capture so it fires before inner handlers but after the opening click settles
    const t = setTimeout(() => window.addEventListener("pointerdown", onPointer, true), 0);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer, true);
      clearTimeout(t);
    };
  }, [onClose, panelRef]);

  return createPortal(
    <div
      ref={panelRef}
      className={cn("material-popover anim-popover fixed z-[70] rounded-[14px] p-1.5", className)}
      style={{
        left: pos?.left ?? -9999,
        top: pos?.top ?? -9999,
        width: matchWidth && anchorRect ? anchorRect.width : undefined,
        // @ts-expect-error custom prop
        "--popover-origin": pos?.origin ?? "top center",
        visibility: pos ? "visible" : "hidden",
      }}
    >
      {children}
    </div>,
    document.body
  );
}

/* =========================================================================
 * Menu — list of actions opened from a trigger
 * ========================================================================= */
export type MenuItem = {
  key: string;
  label: ReactNode;
  icon?: ReactNode;
  onSelect?: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
};

export function Menu({
  trigger,
  items,
  placement = "bottom-start",
  className,
}: {
  trigger: (props: { open: boolean; toggle: (e: React.MouseEvent) => void }) => ReactNode;
  items: MenuItem[];
  placement?: Placement;
  className?: string;
}) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const open = rect !== null;
  const toggle = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    const el = (e.currentTarget as HTMLElement).closest("[data-menu-anchor]") ?? (e.currentTarget as HTMLElement);
    setRect((cur) => (cur ? null : el.getBoundingClientRect()));
  }, []);
  return (
    <span data-menu-anchor className="inline-flex">
      {trigger({ open, toggle })}
      {open && (
        <FloatingPanel anchorRect={rect} placement={placement} onClose={() => setRect(null)} className={cn("min-w-[180px]", className)}>
          <MenuList items={items} onClose={() => setRect(null)} />
        </FloatingPanel>
      )}
    </span>
  );
}

export function MenuList({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  return (
    <>
      {items.map((it) =>
        it.separator ? (
          <div key={it.key} className="my-1 h-px bg-[var(--color-outline-soft)]" />
        ) : (
          <button
            key={it.key}
            data-disabled={it.disabled ? "true" : undefined}
            disabled={it.disabled}
            onClick={() => {
              if (it.disabled) return;
              it.onSelect?.();
              onClose();
            }}
            className={cn(
              "context-menu-item w-full text-left",
              it.danger && "text-[var(--color-destructive)] hover:!bg-[var(--color-destructive)]"
            )}
          >
            {it.icon && <span className="flex size-4 items-center justify-center opacity-80">{it.icon}</span>}
            <span className="flex-1 truncate">{it.label}</span>
          </button>
        )
      )}
    </>
  );
}

/* =========================================================================
 * Dropdown / Select — replaces ugly native <select>
 * ========================================================================= */
export function Dropdown<T extends string | number>({
  value,
  options,
  onChange,
  placeholder = "选择",
  className,
  size = "md",
  placement = "bottom-start",
  disabled,
  matchWidth = true,
}: {
  value: T | null | undefined;
  options: { value: T; label: ReactNode }[];
  onChange: (v: T) => void;
  placeholder?: string;
  className?: string;
  size?: "sm" | "md";
  placement?: Placement;
  disabled?: boolean;
  matchWidth?: boolean;
}) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const open = rect !== null;
  const ref = useRef<HTMLButtonElement>(null);
  const current = options.find((o) => o.value === value);
  const sizeCls = size === "sm" ? "h-8 px-3 text-[13px]" : "h-10 px-3.5 text-sm";
  return (
    <>
      <button
        ref={ref}
        type="button"
        disabled={disabled}
        onClick={() => setRect((cur) => (cur ? null : ref.current!.getBoundingClientRect()))}
        className={cn(
          "inline-flex items-center justify-between gap-2 rounded-[10px] bg-[var(--color-surface-2)] ring-1 ring-inset ring-[var(--color-outline-soft)] text-[var(--color-on-surface)] transition-all hover:ring-[var(--color-outline)] disabled:opacity-40",
          open && "ring-[var(--color-accent)]/45",
          sizeCls,
          className
        )}
      >
        <span className={cn("truncate", !current && "text-[var(--color-on-surface-faint)]")}>
          {current ? current.label : placeholder}
        </span>
        <ChevronDown className={cn("size-4 shrink-0 text-[var(--color-on-surface-faint)] transition-transform", open && "rotate-180")} />
      </button>
      {open && (
        <FloatingPanel anchorRect={rect} placement={placement} onClose={() => setRect(null)} matchWidth={matchWidth} className="min-w-[140px] max-h-[320px] overflow-y-auto">
          {options.map((o) => {
            const active = o.value === value;
            return (
              <button
                key={String(o.value)}
                onClick={() => {
                  onChange(o.value);
                  setRect(null);
                }}
                className={cn(
                  "context-menu-item w-full text-left",
                  active && "font-semibold"
                )}
              >
                <span className="flex-1 truncate">{o.label}</span>
                {active && <CheckIcon className="size-4 text-[var(--color-accent)]" />}
              </button>
            );
          })}
        </FloatingPanel>
      )}
    </>
  );
}

/* =========================================================================
 * Popover — generic anchored floating content
 * ========================================================================= */
export function Popover({
  trigger,
  children,
  placement = "bottom",
  className,
}: {
  trigger: (props: { open: boolean; toggle: (e: React.MouseEvent) => void }) => ReactNode;
  children: (close: () => void) => ReactNode;
  placement?: Placement;
  className?: string;
}) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const open = rect !== null;
  const close = useCallback(() => setRect(null), []);
  const toggle = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    const el = (e.currentTarget as HTMLElement).closest("[data-popover-anchor]") ?? (e.currentTarget as HTMLElement);
    setRect((cur) => (cur ? null : el.getBoundingClientRect()));
  }, []);
  return (
    <span data-popover-anchor className="inline-flex">
      {trigger({ open, toggle })}
      {open && (
        <FloatingPanel anchorRect={rect} placement={placement} onClose={close} className={className}>
          {children(close)}
        </FloatingPanel>
      )}
    </span>
  );
}

/* =========================================================================
 * ContextMenu — right-click menu wrapping arbitrary content
 * ========================================================================= */
export function ContextMenu({ items, children, className }: { items: MenuItem[]; children: ReactNode; className?: string }) {
  const [pt, setPt] = useState<{ x: number; y: number } | null>(null);
  const rect = useMemo(() => (pt ? new DOMRect(pt.x, pt.y, 0, 0) : null), [pt]);
  return (
    <>
      <div
        className={className}
        onContextMenu={(e) => {
          if (!items.length) return;
          e.preventDefault();
          setPt({ x: e.clientX, y: e.clientY });
        }}
      >
        {children}
      </div>
      {pt && (
        <FloatingPanel anchorRect={rect} placement="bottom-start" onClose={() => setPt(null)} className="min-w-[180px]">
          <MenuList items={items} onClose={() => setPt(null)} />
        </FloatingPanel>
      )}
    </>
  );
}

/* =========================================================================
 * Tooltip
 * ========================================================================= */
export function Tooltip({ label, children, placement = "top" }: { label: ReactNode; children: ReactNode; placement?: Placement }) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(null);
  const show = (e: React.MouseEvent) => {
    const el = e.currentTarget as HTMLElement;
    timer.current = setTimeout(() => setRect(el.getBoundingClientRect()), 450);
  };
  const hide = () => {
    if (timer.current) clearTimeout(timer.current);
    setRect(null);
  };
  return (
    <span className="inline-flex" onMouseEnter={show} onMouseLeave={hide}>
      {children}
      {rect &&
        createPortal(
          <Floating tipRect={rect} placement={placement}>
            {label}
          </Floating>,
          document.body
        )}
    </span>
  );
}

function Floating({ tipRect, placement, children }: { tipRect: DOMRect; placement: Placement; children: ReactNode }) {
  const { pos, panelRef } = useAnchoredPosition(tipRect, placement, 8);
  return (
    <div
      ref={panelRef}
      className="anim-popover pointer-events-none fixed z-[85] rounded-lg bg-[rgba(40,40,42,0.95)] px-2.5 py-1.5 text-[12px] font-medium text-white shadow-lg backdrop-blur-md"
      style={{ left: pos?.left ?? -9999, top: pos?.top ?? -9999, visibility: pos ? "visible" : "hidden" }}
    >
      {children}
    </div>
  );
}

/* =========================================================================
 * Dialog / Modal
 * ========================================================================= */
export function Dialog({
  open,
  onClose,
  title,
  children,
  footer,
  className,
}: {
  open: boolean;
  onClose: () => void;
  title?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);
  if (!open) return null;
  return createPortal(
    <div className="fixed inset-0 z-[75] flex items-center justify-center p-6">
      <div className="anim-dialog-backdrop absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div
        className={cn(
          "anim-dialog-panel material-popover relative z-[1] w-full max-w-md rounded-[20px] p-5",
          className
        )}
      >
        {title && <div className="mb-3 text-[17px] font-semibold text-[var(--color-on-surface)]">{title}</div>}
        <div className="text-[14px] text-[var(--color-on-surface-muted)]">{children}</div>
        {footer && <div className="mt-5 flex justify-end gap-2">{footer}</div>}
      </div>
    </div>,
    document.body
  );
}

/* =========================================================================
 * EmptyState
 * ========================================================================= */
export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
}: {
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-3 px-6 py-14 text-center", className)}>
      {icon && (
        <div className="flex size-14 items-center justify-center rounded-2xl bg-[var(--color-surface-3)] text-[var(--color-on-surface-faint)]">
          {icon}
        </div>
      )}
      <div className="text-[15px] font-semibold text-[var(--color-on-surface)]">{title}</div>
      {description && <div className="max-w-xs text-[13px] leading-relaxed text-[var(--color-on-surface-faint)]">{description}</div>}
      {action && <div className="mt-1">{action}</div>}
    </div>
  );
}

/* =========================================================================
 * Dashboard primitives — StatTile / RingChart / BarChart
 * ========================================================================= */
export function StatTile({
  label,
  value,
  hint,
  icon,
  accent,
  className,
}: {
  label: ReactNode;
  value: ReactNode;
  hint?: ReactNode;
  icon?: ReactNode;
  accent?: boolean;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-1 rounded-[var(--radius-card)] bg-[var(--color-surface-1)] p-4 ring-1 ring-inset ring-[var(--color-outline-soft)]",
        className
      )}
    >
      <div className="flex items-center gap-2 text-[12px] font-medium text-[var(--color-on-surface-faint)]">
        {icon && <span className={cn("flex size-4 items-center justify-center", accent && "text-[var(--color-accent)]")}>{icon}</span>}
        {label}
      </div>
      <div className="text-[26px] font-semibold leading-none tracking-tight tabular-nums text-[var(--color-on-surface)]">{value}</div>
      {hint && <div className="text-[12px] text-[var(--color-on-surface-faint)]">{hint}</div>}
    </div>
  );
}

export function RingChart({
  segments,
  size = 132,
  thickness = 14,
  centerLabel,
  centerSub,
}: {
  segments: { value: number; color: string; label?: string }[];
  size?: number;
  thickness?: number;
  centerLabel?: ReactNode;
  centerSub?: ReactNode;
}) {
  const total = segments.reduce((s, x) => s + x.value, 0) || 1;
  const r = (size - thickness) / 2;
  const c = 2 * Math.PI * r;
  let acc = 0;
  return (
    <div className="relative inline-flex items-center justify-center" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--color-surface-3)" strokeWidth={thickness} />
        {segments.map((seg, i) => {
          const frac = seg.value / total;
          const dash = frac * c;
          const el = (
            <circle
              key={i}
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke={seg.color}
              strokeWidth={thickness}
              strokeLinecap="round"
              strokeDasharray={`${Math.max(0, dash - 2)} ${c}`}
              strokeDashoffset={-acc * c}
              style={{ transition: "stroke-dasharray 0.6s cubic-bezier(0.32,0.72,0,1)" }}
            />
          );
          acc += frac;
          return el;
        })}
      </svg>
      {(centerLabel || centerSub) && (
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          {centerLabel && <div className="text-[22px] font-semibold leading-none tabular-nums text-[var(--color-on-surface)]">{centerLabel}</div>}
          {centerSub && <div className="mt-1 text-[11px] text-[var(--color-on-surface-faint)]">{centerSub}</div>}
        </div>
      )}
    </div>
  );
}

export function BarChart({
  data,
  height = 120,
  className,
}: {
  data: { label: string; value: number; color?: string }[];
  height?: number;
  className?: string;
}) {
  const max = Math.max(1, ...data.map((d) => d.value));
  return (
    <div className={cn("flex items-end gap-1.5", className)} style={{ height }}>
      {data.map((d, i) => (
        <div key={i} className="flex flex-1 flex-col items-center gap-1.5">
          <div className="flex w-full flex-1 items-end">
            <div
              className="w-full rounded-t-[5px] transition-all duration-500"
              style={{
                height: `${(d.value / max) * 100}%`,
                minHeight: d.value > 0 ? 4 : 0,
                background: d.color ?? "var(--color-accent)",
              }}
              title={`${d.label}: ${d.value}`}
            />
          </div>
          <span className="text-[10px] tabular-nums text-[var(--color-on-surface-faint)]">{d.label}</span>
        </div>
      ))}
    </div>
  );
}

/* =========================================================================
 * BootSplash — first-load animation
 * ========================================================================= */
export function BootSplash({ leaving }: { leaving: boolean }) {
  return (
    <div className={cn("boot-splash", leaving && "is-leaving")}>
      <div className="boot-splash-mark flex flex-col items-center gap-5">
        <div className="flex size-[72px] items-center justify-center rounded-[20px] bg-[var(--color-accent)] text-[34px] font-bold text-white shadow-lg shadow-[var(--color-accent)]/30">
          N
        </div>
        <div className="text-[15px] font-medium tracking-wide text-[var(--color-on-surface-muted)]">NexPlay</div>
      </div>
      <Spinner size={20} className="text-[var(--color-on-surface-faint)]" />
    </div>
  );
}
