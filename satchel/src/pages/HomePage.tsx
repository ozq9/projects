import clsx from "clsx";
import { Check, KeyRound, PlugZap } from "lucide-react";
import type { ReactNode } from "react";
import { useNavigate } from "react-router";

import { Button, Notice, PageHeader } from "../components/ui";
import { formatSeconds } from "../lib/format";
import { errorHint, useConnectionTest, useStatus, useUsage } from "../lib/hooks";

type StepState = "done" | "current" | "upcoming";

function Step({ index, state, title, children }: { index: number; state: StepState; title: string; children?: ReactNode }) {
  return (
    <li className="group relative flex gap-4 pb-7 last:pb-0">
      {/* The rail connecting the steps. */}
      <span aria-hidden className="absolute top-9 bottom-1 left-[15px] w-px bg-slate-line group-last:hidden" />
      <span
        className={clsx(
          "relative z-10 grid size-8 shrink-0 place-items-center rounded-full text-[0.85rem] font-semibold tabular-nums",
          state === "done" && "bg-ok/15 text-ok",
          state === "current" && "bg-highlight text-highlight-ink",
          state === "upcoming" && "bg-slate-lift text-chalk-mute",
        )}
      >
        {state === "done" ? <Check className="size-4" strokeWidth={3} aria-label="Done" /> : index}
      </span>
      <div className="min-w-0 flex-1 pt-1">
        <h3 className={clsx("text-[1.02rem] font-semibold", state === "upcoming" ? "text-chalk-mute" : "text-chalk")}>{title}</h3>
        {children && <div className="mt-1.5 space-y-3 text-[0.92rem] text-chalk-soft">{children}</div>}
      </div>
    </li>
  );
}

export function HomePage() {
  const navigate = useNavigate();
  const status = useStatus();
  const usage = useUsage();
  const test = useConnectionTest();

  const hasKey = !!status.data?.apiKey.present;
  const testedBefore = !!usage.data?.byPurpose.some((g) => g.key === "connection_test" && g.calls > 0);
  const tested = test.isSuccess || (testedBefore && !test.isError);
  const allDone = hasKey && tested;

  return (
    <div className="appear">
      <PageHeader
        title={allDone ? "Satchel is ready." : "Let's set up Satchel."}
        lead={
          allDone
            ? "Your library and your Claude connection both work. Dropping in worksheets, notes and slides comes next."
            : "Three quick steps, then Satchel can start reading your schoolwork."
        }
      />

      <ol className="max-w-[44rem] rounded-[var(--radius-panel)] bg-slate-raised px-6 py-6">
        <Step index={1} state={status.data ? "done" : "current"} title="Your library is ready">
          {status.data && (
            <p>
              Everything you add is kept in <span className="selectable break-all text-chalk">{status.data.dataDir}</span>
            </p>
          )}
        </Step>

        <Step index={2} state={hasKey ? "done" : "current"} title={hasKey ? "Claude API key saved" : "Add your Claude API key"}>
          {hasKey ? (
            <p>
              Stored in {status.data?.keyStore.name} as <span className="text-chalk">{status.data?.apiKey.hint}</span>.
            </p>
          ) : (
            <>
              <p>Satchel uses your own key to read and solve your schoolwork. You pay Claude directly, per use.</p>
              <Button variant="primary" icon={KeyRound} onClick={() => navigate("/settings")}>
                Add API key
              </Button>
            </>
          )}
        </Step>

        <Step index={3} state={tested ? "done" : hasKey ? "current" : "upcoming"} title={tested ? "Connection works" : "Test the connection"}>
          {test.isSuccess && (
            <p>
              “{test.data.greeting}”{" "}
              <span className="text-chalk-mute">Answered in {formatSeconds(test.data.latencyMs)}.</span>
            </p>
          )}
          {!test.isSuccess && testedBefore && !test.isError && <p>The last test succeeded.</p>}
          {hasKey && !tested && (
            <>
              <p>Sends one tiny request to Claude to check the key and model. It costs a fraction of a cent.</p>
              {test.isError && (
                <Notice tone="error">
                  {test.error.message} {errorHint(test.error)}
                </Notice>
              )}
              <Button variant="primary" icon={PlugZap} busy={test.isPending} onClick={() => test.mutate()}>
                {test.isPending ? "Testing…" : "Test connection"}
              </Button>
            </>
          )}
        </Step>
      </ol>

      {status.isError && (
        <div className="mt-6 max-w-[44rem]">
          <Notice tone="error">{status.error.message}</Notice>
        </div>
      )}
    </div>
  );
}
