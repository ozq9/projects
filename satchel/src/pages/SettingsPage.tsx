import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ExternalLink, FolderInput, FolderOpen, KeyRound, PlugZap, RotateCcw } from "lucide-react";
import { useEffect, useId, useState, type FormEvent } from "react";
import { useLocation } from "react-router";

import { Button, Notice, PageHeader, Section } from "../components/ui";
import { api, queryKeys } from "../lib/api";
import { formatSeconds, formatTokens, formatUsd, formatUsdPrecise, purposeLabel } from "../lib/format";
import { errorHint, useConnectionTest, useStatus, useUsage } from "../lib/hooks";
import type { AppStatus, UsageTotals } from "../lib/types";

export function SettingsPage() {
  const status = useStatus();
  const { hash } = useLocation();

  // Support links like /settings#usage from the sidebar.
  useEffect(() => {
    if (hash && status.data) document.getElementById(hash.slice(1))?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [hash, status.data]);

  return (
    <div className="appear max-w-[64rem]">
      <PageHeader title="Settings" />
      {status.isError && <Notice tone="error">{status.error.message}</Notice>}
      {status.data && (
        <div>
          <Section id="api-key" title="Claude API key" description="Satchel uses your own key for everything it does with Claude. You pay Claude directly, per use.">
            <ApiKeyForm status={status.data} />
          </Section>
          <Section id="model" title="Model" description="The Claude model Satchel uses for every task.">
            <ModelForm current={status.data.settings.model} />
          </Section>
          <Section id="connection" title="Connection" description="Sends one tiny request to check that the key and model work.">
            <ConnectionCheck hasKey={status.data.apiKey.present} />
          </Section>
          <Section id="usage" title="Usage" description="Tokens used and the estimated cost, based on Claude's list prices.">
            <UsagePanel />
          </Section>
          <Section id="library" title="Library folder" description="Where Satchel keeps its database and the copies of your files.">
            <LibraryFolder status={status.data} />
          </Section>
          <p className="pt-2 pb-10 text-[0.8rem] text-chalk-mute">
            Satchel {status.data.version}, library format {status.data.schemaVersion}
          </p>
        </div>
      )}
    </div>
  );
}

function ApiKeyForm({ status }: { status: AppStatus }) {
  const queryClient = useQueryClient();
  const inputId = useId();
  const [draft, setDraft] = useState("");
  const [replacing, setReplacing] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const refresh = () => queryClient.invalidateQueries({ queryKey: queryKeys.status });
  const save = useMutation({
    mutationFn: api.setApiKey,
    onSuccess: () => {
      setDraft("");
      setReplacing(false);
      refresh();
    },
  });
  const remove = useMutation({
    mutationFn: api.deleteApiKey,
    onSuccess: () => {
      setConfirmRemove(false);
      refresh();
    },
  });

  const { keyStore, apiKey } = status;
  const showForm = !apiKey.present || replacing;

  const submit = (e: FormEvent) => {
    e.preventDefault();
    save.mutate(draft);
  };

  return (
    <>
      {keyStore.error && <Notice tone="error">{keyStore.name} isn't available: {keyStore.error}</Notice>}
      {!keyStore.persistent && (
        <Notice tone="info">This is a development build. The key is kept in memory and forgotten when Satchel closes.</Notice>
      )}

      {apiKey.present && !replacing && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-xl bg-slate-raised px-4 py-3">
          <KeyRound className="size-4 text-ok" aria-hidden />
          <div className="mr-auto min-w-0">
            <div className="text-chalk">{apiKey.hint}</div>
            <div className="text-[0.82rem] text-chalk-mute">Saved in {keyStore.name}</div>
          </div>
          {!confirmRemove && (
            <div className="flex gap-1">
              <Button variant="ghost" onClick={() => setReplacing(true)}>
                Replace
              </Button>
              <Button variant="danger" onClick={() => setConfirmRemove(true)}>
                Remove
              </Button>
            </div>
          )}
        </div>
      )}

      {confirmRemove && (
        <Notice
          tone="info"
          action={
            <div className="flex gap-2">
              <Button variant="danger" className="bg-danger/10" busy={remove.isPending} onClick={() => remove.mutate()}>
                Remove key
              </Button>
              <Button variant="ghost" onClick={() => setConfirmRemove(false)}>
                Keep it
              </Button>
            </div>
          }
        >
          Remove the saved key? Satchel can't reach Claude until you add one again.
        </Notice>
      )}
      {remove.isError && <Notice tone="error">{remove.error.message}</Notice>}

      {showForm && (
        <form onSubmit={submit} className="space-y-3">
          <label htmlFor={inputId} className="block text-[0.88rem] text-chalk-soft">
            {replacing ? "New API key" : "API key"}
          </label>
          <div className="flex gap-2">
            <input
              id={inputId}
              className="field"
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder="sk-ant-…"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
            />
            <Button type="submit" variant="primary" busy={save.isPending} disabled={!draft.trim()}>
              Save key
            </Button>
            {replacing && (
              <Button variant="ghost" onClick={() => setReplacing(false)}>
                Cancel
              </Button>
            )}
          </div>
          {save.isError && <Notice tone="error">{save.error.message}</Notice>}
          <Button variant="ghost" icon={ExternalLink} className="-ml-3.5" onClick={() => api.openApiKeysPage()}>
            Create a key in the Claude Console
          </Button>
        </form>
      )}
    </>
  );
}

function ModelForm({ current }: { current: string }) {
  const queryClient = useQueryClient();
  const inputId = useId();
  const listId = useId();
  const [draft, setDraft] = useState(current);
  const models = useQuery({ queryKey: queryKeys.models, queryFn: api.models, staleTime: Infinity });

  useEffect(() => setDraft(current), [current]);

  const save = useMutation({
    mutationFn: (model: string) => api.saveSettings({ model }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.status }),
  });

  const trimmed = draft.trim();
  const known = models.data?.find((m) => m.id === trimmed);
  const fallback = models.data?.find((m) => m.isDefault);
  const dirty = trimmed !== current;

  return (
    <form
      className="space-y-3"
      onSubmit={(e) => {
        e.preventDefault();
        save.mutate(trimmed);
      }}
    >
      <label htmlFor={inputId} className="block text-[0.88rem] text-chalk-soft">
        Model name
      </label>
      <div className="flex gap-2">
        <input
          id={inputId}
          className="field"
          list={listId}
          spellCheck={false}
          autoComplete="off"
          value={draft}
          onChange={(e) => {
            setDraft(e.target.value);
            save.reset();
          }}
        />
        <datalist id={listId}>
          {models.data?.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </datalist>
        <Button type="submit" busy={save.isPending} disabled={!dirty || !trimmed}>
          Save model
        </Button>
      </div>

      <p className="text-[0.88rem] text-chalk-mute">
        {known
          ? `${known.name} costs $${known.inputPerMtok} per million input tokens and $${known.outputPerMtok} per million output tokens.`
          : trimmed
            ? "Satchel doesn't know this model's price, so its calls are counted in tokens but not in the cost total."
            : "Enter a model name."}
      </p>

      {save.isError && <Notice tone="error">{save.error.message}</Notice>}
      {save.isSuccess && !dirty && <Notice tone="ok">Model saved. Satchel uses {current} from now on.</Notice>}
      {fallback && trimmed !== fallback.id && (
        <Button variant="ghost" icon={RotateCcw} className="-ml-3.5" onClick={() => setDraft(fallback.id)}>
          Use the recommended model ({fallback.name})
        </Button>
      )}
    </form>
  );
}

function ConnectionCheck({ hasKey }: { hasKey: boolean }) {
  const test = useConnectionTest();
  const result = test.data;

  return (
    <>
      <Button icon={PlugZap} busy={test.isPending} disabled={!hasKey} onClick={() => test.mutate()}>
        {test.isPending ? "Testing…" : "Test connection"}
      </Button>
      {!hasKey && <p className="text-[0.88rem] text-chalk-mute">Save an API key first.</p>}

      {test.isError && (
        <Notice tone="error">
          {test.error.message} {errorHint(test.error)}
        </Notice>
      )}

      {result && (
        <Notice tone="ok">
          <p className="text-chalk">“{result.greeting}”</p>
          <p className="mt-1 text-[0.85rem] text-chalk-mute">
            Answered by {result.modelServed} in {formatSeconds(result.latencyMs)}, using {formatTokens(result.inputTokens)} input and{" "}
            {formatTokens(result.outputTokens)} output tokens
            {result.costUsd != null ? ` (${formatUsdPrecise(result.costUsd)})` : ""}.
            {result.modelServed !== result.modelRequested &&
              ` ${result.modelRequested} declined the request, so Claude answered with ${result.modelServed} instead.`}
          </p>
        </Notice>
      )}
    </>
  );
}

function Totals({ label, totals }: { label: string; totals: UsageTotals }) {
  return (
    <div className="rounded-xl bg-slate-raised px-4 py-3.5">
      <div className="text-[0.85rem] text-chalk-mute">{label}</div>
      <div className="mt-0.5 font-display text-[1.6rem] font-semibold tabular-nums tracking-[-0.02em]">{formatUsd(totals.costUsd)}</div>
      <div className="text-[0.82rem] text-chalk-mute tabular-nums">
        {totals.calls} {totals.calls === 1 ? "call" : "calls"}, {formatTokens(totals.inputTokens + totals.cacheReadTokens + totals.cacheCreationTokens)} in,{" "}
        {formatTokens(totals.outputTokens)} out
      </div>
    </div>
  );
}

function UsagePanel() {
  const usage = useUsage();
  if (usage.isError) return <Notice tone="error">{usage.error.message}</Notice>;
  if (!usage.data) return null;
  const { thisMonth, allTime, byPurpose } = usage.data;

  if (allTime.calls === 0) {
    return <p className="text-[0.92rem] text-chalk-soft">No Claude calls yet. Costs show up here as Satchel works.</p>;
  }

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2">
        <Totals label="This month" totals={thisMonth} />
        <Totals label="All time" totals={allTime} />
      </div>
      <table className="w-full text-[0.88rem] tabular-nums">
        <caption className="sr-only">Usage by task</caption>
        <thead>
          <tr className="text-left text-chalk-mute">
            <th className="py-1.5 font-normal">Task</th>
            <th className="py-1.5 text-right font-normal">Calls</th>
            <th className="py-1.5 text-right font-normal">Tokens</th>
            <th className="py-1.5 text-right font-normal">Cost</th>
          </tr>
        </thead>
        <tbody>
          {byPurpose.map((g) => (
            <tr key={g.key} className="border-t border-slate-line/60 text-chalk-soft">
              <td className="py-2">{purposeLabel(g.key)}</td>
              <td className="py-2 text-right">{g.calls}</td>
              <td className="py-2 text-right">{formatTokens(g.inputTokens + g.outputTokens + g.cacheReadTokens + g.cacheCreationTokens)}</td>
              <td className="py-2 text-right">{formatUsd(g.costUsd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {allTime.unpricedCalls > 0 && (
        <p className="text-[0.85rem] text-chalk-mute">
          {allTime.unpricedCalls} {allTime.unpricedCalls === 1 ? "call used a model" : "calls used models"} with an unknown price, not included in the
          cost.
        </p>
      )}
    </>
  );
}

function LibraryFolder({ status }: { status: AppStatus }) {
  const [target, setTarget] = useState<string | null>(null);
  const pick = useMutation({ mutationFn: api.pickFolder, onSuccess: (path) => path && setTarget(path) });
  const move = useMutation({ mutationFn: api.moveDataDir });
  const open = useMutation({ mutationFn: api.openDataDir });

  return (
    <>
      <div className="rounded-xl bg-slate-well px-4 py-3">
        <div className="selectable break-all text-chalk">{status.dataDir}</div>
        <div className="mt-0.5 text-[0.82rem] text-chalk-mute">{status.isCustomDataDir ? "Custom location" : "Default location"}</div>
      </div>

      {!target && (
        <div className="flex flex-wrap gap-2">
          <Button icon={FolderOpen} onClick={() => open.mutate()}>
            Open folder
          </Button>
          <Button variant="ghost" icon={FolderInput} busy={pick.isPending} onClick={() => pick.mutate()}>
            Move library…
          </Button>
        </div>
      )}
      {open.isError && <Notice tone="error">{open.error.message}</Notice>}
      {pick.isError && <Notice tone="error">{pick.error.message}</Notice>}

      {target && !move.isSuccess && (
        <Notice
          tone="info"
          action={
            <div className="flex gap-2">
              <Button variant="primary" busy={move.isPending} onClick={() => move.mutate(target)}>
                Move and restart
              </Button>
              <Button
                variant="ghost"
                disabled={move.isPending}
                onClick={() => {
                  setTarget(null);
                  move.reset();
                }}
              >
                Cancel
              </Button>
            </div>
          }
        >
          <p>
            Move your library to <span className="break-all text-chalk">{target}</span>?
          </p>
          <p className="mt-1">
            Satchel copies everything there and restarts. The current folder stays where it is as a backup; delete it yourself once you've checked
            that everything arrived.
          </p>
        </Notice>
      )}
      {move.isError && <Notice tone="error">{move.error.message}</Notice>}
      {move.isSuccess && <Notice tone="ok">Library moved. Restarting…</Notice>}
    </>
  );
}
