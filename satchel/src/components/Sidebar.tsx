import { useQuery } from "@tanstack/react-query";
import clsx from "clsx";
import { House, Settings2, type LucideIcon } from "lucide-react";
import { NavLink } from "react-router";

import { api, queryKeys } from "../lib/api";
import { formatUsd } from "../lib/format";
import { Logo } from "./Logo";

const nav: { to: string; label: string; icon: LucideIcon }[] = [
  { to: "/", label: "Home", icon: House },
  { to: "/settings", label: "Settings", icon: Settings2 },
];

export function Sidebar() {
  const usage = useQuery({ queryKey: queryKeys.usage, queryFn: api.usage });
  const month = usage.data?.thisMonth;

  return (
    <aside className="flex w-56 shrink-0 flex-col bg-slate-side px-3 pt-5 pb-4">
      <div className="mb-7 flex items-center gap-2.5 px-2.5">
        <Logo className="size-7" />
        <span className="font-display text-[1.3rem] font-semibold tracking-[-0.02em]">Satchel</span>
      </div>

      <nav aria-label="Main" className="flex flex-col gap-0.5">
        {nav.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              clsx(
                "group flex items-center gap-3 rounded-[var(--radius-control)] px-2.5 py-2 text-[0.95rem] transition-colors duration-150",
                isActive ? "text-chalk" : "text-chalk-mute hover:bg-slate-lift/60 hover:text-chalk-soft",
              )
            }
          >
            {({ isActive }) => (
              <>
                <Icon className={clsx("size-[1.05rem]", isActive && "text-highlight")} aria-hidden />
                <span className={clsx(isActive && "highlighter")}>{label}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="mt-auto px-2.5">
        {month && (
          <NavLink to="/settings#usage" className="block rounded-[var(--radius-control)] py-2 text-[0.82rem] text-chalk-mute hover:text-chalk-soft">
            <span className="block text-chalk-soft tabular-nums">{formatUsd(month.costUsd)} this month</span>
            <span>
              {month.calls} Claude {month.calls === 1 ? "call" : "calls"}
            </span>
          </NavLink>
        )}
      </div>
    </aside>
  );
}
