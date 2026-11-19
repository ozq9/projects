import clsx from "clsx";
import { CircleAlert, CircleCheck, Info, LoaderCircle, type LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const variants: Record<Variant, string> = {
  // A disabled primary turns neutral instead of a faded (muddy) yellow.
  primary:
    "bg-highlight text-highlight-ink hover:bg-highlight-strong font-semibold shadow-[0_1px_0_0_rgba(255,255,255,0.25)_inset] disabled:bg-slate-lift disabled:text-chalk-mute disabled:shadow-none",
  secondary: "bg-slate-lift text-chalk hover:bg-slate-line disabled:opacity-50",
  ghost: "text-chalk-soft hover:text-chalk hover:bg-slate-lift/70 disabled:opacity-50",
  danger: "text-danger hover:bg-danger/10 disabled:opacity-50",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  icon?: LucideIcon;
  busy?: boolean;
}

export function Button({ variant = "secondary", icon: Icon, busy, className, children, disabled, ...rest }: ButtonProps) {
  return (
    <button
      type="button"
      {...rest}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
      className={clsx(
        "inline-flex h-9 items-center gap-2 rounded-[var(--radius-control)] px-3.5 text-[0.92rem] whitespace-nowrap",
        "transition-colors duration-150 disabled:cursor-not-allowed",
        variants[variant],
        className,
      )}
    >
      {busy ? <LoaderCircle className="size-4 animate-spin" aria-hidden /> : Icon ? <Icon className="size-4" aria-hidden /> : null}
      {children}
    </button>
  );
}

type Tone = "error" | "ok" | "info";

const toneStyles: Record<Tone, { box: string; icon: LucideIcon; iconClass: string }> = {
  error: { box: "bg-danger/8 text-chalk", icon: CircleAlert, iconClass: "text-danger" },
  ok: { box: "bg-ok/8 text-chalk", icon: CircleCheck, iconClass: "text-ok" },
  info: { box: "bg-slate-lift/60 text-chalk-soft", icon: Info, iconClass: "text-chalk-mute" },
};

export function Notice({ tone, children, action }: { tone: Tone; children: ReactNode; action?: ReactNode }) {
  const { box, icon: Icon, iconClass } = toneStyles[tone];
  return (
    <div role={tone === "error" ? "alert" : "status"} className={clsx("appear flex gap-3 rounded-xl px-4 py-3 text-[0.92rem]", box)}>
      <Icon className={clsx("mt-0.5 size-[1.1rem] shrink-0", iconClass)} aria-hidden />
      <div className="min-w-0 flex-1 space-y-2">
        <div className="selectable">{children}</div>
        {action}
      </div>
    </div>
  );
}

/** One settings group: label column on the left, controls on the right. */
export function Section({ id, title, description, children }: { id?: string; title: string; description: ReactNode; children: ReactNode }) {
  return (
    <section id={id} className="grid gap-x-12 gap-y-4 border-t border-slate-line/70 py-9 first:border-t-0 first:pt-2 lg:grid-cols-[15rem_minmax(0,1fr)]">
      <div>
        <h2 className="font-display text-[1.15rem] font-semibold tracking-[-0.01em] text-chalk">{title}</h2>
        <p className="mt-1.5 max-w-[38ch] text-[0.88rem] leading-relaxed text-chalk-mute">{description}</p>
      </div>
      <div className="min-w-0 max-w-[40rem] space-y-4">{children}</div>
    </section>
  );
}

export function PageHeader({ title, lead }: { title: string; lead?: ReactNode }) {
  return (
    <header className="mb-8">
      <h1 className="font-display text-[2.5rem] leading-[1.05] font-semibold tracking-[-0.025em] text-chalk [font-variation-settings:'opsz'_96]">
        {title}
      </h1>
      {lead && <p className="mt-3 max-w-[60ch] text-[1.02rem] text-chalk-soft">{lead}</p>}
    </header>
  );
}
