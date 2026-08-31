import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { Application, ApplyKit, AutoUpdateSettings, Lead } from "../types";
import { timeAgo } from "../lib/format";
import { Badge, displayLabel, statusTone } from "../components/Badge";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Modal from "../components/Modal";
import ScorePill from "../components/ScorePill";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

type Tab = "discover" | "due";

const SOURCE_LABEL: Record<string, string> = {
  indeed: "Indeed",
  linkedin: "LinkedIn",
  upwork: "Upwork",
  fiverr: "Fiverr",
  freelancer: "Freelancer",
  facebook: "Facebook",
  manual: "Manual",
};

function sourceLabel(source: string): string {
  return SOURCE_LABEL[source] ?? source;
}

function mediumLabel(medium: string): string {
  return medium === "proposal"
    ? "Proposal"
    : medium === "linkedin_message"
      ? "LinkedIn message"
      : "Email";
}

export default function Discover() {
  const [tab, setTab] = useState<Tab>("discover");
  const [settings, setSettings] = useState<AutoUpdateSettings | null>(null);
  const [queue, setQueue] = useState<Lead[]>([]);
  const [due, setDue] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [kit, setKit] = useState<ApplyKit | null>(null);
  const [kitLoading, setKitLoading] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const { notify } = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [s, q, d] = await Promise.all([
        api.getAutoUpdateSettings(),
        api.queuedLeads(),
        api.applicationsDue(),
      ]);
      setSettings(s);
      setQueue(q);
      setDue(d);
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to load discovery data", "error");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const saveSettings = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await api.saveAutoUpdateSettings({
        enabled: settings.enabled,
        interval_mins: settings.interval_mins,
        threshold: settings.threshold,
      });
      notify("Auto-discovery settings saved");
      await api.rescoreLeads();
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save settings", "error");
    } finally {
      setSaving(false);
    }
  };

  const runNow = async () => {
    setRunning(true);
    try {
      const r = await api.scrape({ sources: ["indeed"] });
      notify(`Discovery complete: ${r.inserted} new leads`);
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Discovery failed", "error");
    } finally {
      setRunning(false);
    }
  };

  const openKit = async (lead: Lead) => {
    setKitLoading(true);
    try {
      setKit(await api.applyKit(lead.id));
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to build application kit", "error");
    } finally {
      setKitLoading(false);
    }
  };

  const copy = async (text: string, key: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(key);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      notify("Could not copy to clipboard", "error");
    }
  };

  const followUp = async (app: Application) => {
    try {
      await api.updateApplication(app.id, { follow_up: true });
      notify("Follow-up logged");
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Follow-up failed", "error");
    }
  };

  if (loading || !settings) {
    return (
      <div className="grid place-items-center py-24">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-slate-900">Discover & apply</h1>
        <p className="mt-1 text-sm text-slate-500">
          Auto-finds high-fit work, drafts your tailored application, and tells you exactly whom to follow up with.
        </p>
      </div>

      <div className="flex gap-2">
        {(
          [
            { id: "discover", label: "Discovery & queue" },
            { id: "due", label: "Follow-up due" },
          ] as { id: Tab; label: string }[]
        ).map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
              tab === t.id
                ? "bg-indigo-600 text-white"
                : "text-slate-600 hover:bg-slate-100"
            }`}
          >
            {t.label}
            {t.id === "due" && due.length > 0 && (
              <span className="ml-1.5 rounded-full bg-amber-100 px-1.5 text-[11px] font-bold text-amber-700">
                {due.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {tab === "discover" && (
        <div className="space-y-5">
          <section className="rounded-xl border border-slate-200 bg-white p-5">
            <div className="flex flex-wrap items-center gap-4">
              <div className="flex items-center gap-3">
                <label className="flex items-center gap-2 text-sm font-medium text-slate-700">
                  <input
                    type="checkbox"
                    checked={settings.enabled}
                    onChange={(e) =>
                      setSettings({ ...settings, enabled: e.target.checked })
                    }
                    className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                  />
                  Auto-discovery
                </label>
                <label className="flex items-center gap-2 text-sm text-slate-600">
                  every
                  <input
                    type="number"
                    min={10}
                    value={settings.interval_mins}
                    onChange={(e) =>
                      setSettings({ ...settings, interval_mins: Number(e.target.value) })
                    }
                    className="w-20 rounded-lg border border-slate-300 px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  />
                  min
                </label>
                <label className="flex items-center gap-2 text-sm text-slate-600">
                  queue at score
                  <input
                    type="number"
                    min={0}
                    max={100}
                    value={settings.threshold}
                    onChange={(e) =>
                      setSettings({ ...settings, threshold: Number(e.target.value) })
                    }
                    className="w-16 rounded-lg border border-slate-300 px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  />
                </label>
              </div>
              <div className="ml-auto flex items-center gap-2">
                <Button variant="secondary" onClick={runNow} loading={running} icon={<Icon name="refresh" className="h-4 w-4" />}>
                  Scan now
                </Button>
                <Button onClick={saveSettings} loading={saving} icon={<Icon name="check" className="h-4 w-4" />}>
                  Save
                </Button>
              </div>
            </div>
            <p className="mt-3 text-xs text-slate-500">
              {settings.enabled
                ? `Runs every ${settings.interval_mins} min against Indeed, scores new leads, and queues anything scoring ${settings.threshold}+.`
                : "Auto-discovery is off. Turn it on to continuously pull new high-fit work."}
              {settings.last_pull && (
                <span className="ml-2 text-slate-400">Last scan: {timeAgo(settings.last_pull)}</span>
              )}
            </p>
          </section>

          <section>
            <div className="mb-2 flex items-center gap-2">
              <h2 className="text-base font-semibold text-slate-900">High-fit queue</h2>
              <Badge tone="emerald">{queue.length}</Badge>
            </div>
            {queue.length === 0 ? (
              <EmptyState
                icon="search"
                title="No high-fit leads queued yet"
                hint="Turn on auto-discovery or run a scan. Leads scoring at or above your threshold will appear here, each with a one-click tailored application."
                action={
                  <Button variant="secondary" onClick={runNow} loading={running}>
                    Scan Indeed now
                  </Button>
                }
              />
            ) : (
              <ul className="divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white">
                {queue.map((lead) => (
                  <li key={lead.id} className="flex items-start gap-3 p-4">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-semibold text-slate-900">{lead.title}</p>
                      <p className="mt-0.5 text-xs text-slate-500">
                        {sourceLabel(lead.source)}
                        {lead.location ? ` · ${lead.location}` : ""}
                        {lead.posted_date ? ` · ${timeAgo(lead.posted_date)}` : ""}
                      </p>
                    </div>
                    <div className="flex shrink-0 items-center gap-2">
                      <ScorePill score={lead.score} />
                      <Button size="sm" onClick={() => openKit(lead)} icon={<Icon name="send" className="h-3.5 w-3.5" />}>
                        Apply
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      )}

      {tab === "due" && (
        <section>
          <div className="mb-2 flex items-center gap-2">
            <h2 className="text-base font-semibold text-slate-900">Follow-up due</h2>
            <Badge tone="amber">{due.length}</Badge>
          </div>
          {due.length === 0 ? (
            <EmptyState
              icon="check"
              title="Nothing needs a follow-up"
              hint="Any application still in the pipeline that's gone quiet will show up here so you never let a thread go cold."
            />
          ) : (
            <ul className="divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white">
              {due.map((app) => (
                <li key={app.id} className="flex items-start gap-3 p-4">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-semibold text-slate-900">
                      {app.lead_title ?? app.company ?? "Application"}
                    </p>
                    <p className="mt-0.5 text-xs text-slate-500">
                      <Badge tone={statusTone(app.status)}>{displayLabel(app.status)}</Badge>
                      {app.applied_at && <span> · applied {timeAgo(app.applied_at)}</span>}
                      {app.follow_up_count > 0 && (
                        <span> · {app.follow_up_count} follow-up{app.follow_up_count > 1 ? "s" : ""}</span>
                      )}
                      {app.next_scheduled && <span> · scheduled {timeAgo(app.next_scheduled)}</span>}
                    </p>
                  </div>
                  <div className="flex shrink-0 gap-2">
                    {app.lead_url && (
                      <a
                        href={app.lead_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs font-medium text-slate-600 ring-1 ring-inset ring-slate-300 transition-colors hover:bg-slate-50"
                      >
                        <Icon name="external" className="h-3.5 w-3.5" />
                        Open
                      </a>
                    )}
                    <Button size="sm" variant="secondary" onClick={() => followUp(app)}>
                      Log follow-up
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <Modal
        open={Boolean(kit)}
        title="Application kit"
        onClose={() => setKit(null)}
        footer={
          kit && (
            <>
              <Button variant="ghost" onClick={() => setKit(null)}>
                Close
              </Button>
              <a
                href={kit.apply_url}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center justify-center rounded-lg bg-indigo-600 px-3.5 py-2 text-sm font-medium text-white transition-colors hover:bg-indigo-500"
              >
                <Icon name="external" className="mr-1.5 h-4 w-4" />
                Open application
              </a>
            </>
          )
        }
      >
        {kitLoading || !kit ? (
          <div className="grid place-items-center py-10">
            <Spinner className="h-6 w-6" />
          </div>
        ) : (
          <div className="space-y-4">
            <div>
              <p className="text-sm font-semibold text-slate-900">{kit.lead.title}</p>
              <p className="text-xs text-slate-500">
                {sourceLabel(kit.source)} · score {kit.lead.score}
              </p>
            </div>
            <p className="rounded-lg bg-slate-50 p-3 text-xs leading-relaxed text-slate-600">
              This is a <strong>review-and-confirm</strong> kit. The tailored copy below is pre-drafted from your
              profile. Open the application on the source site, paste the copy, review it, and submit yourself. Nothing
              is auto-submitted for you.
            </p>
            <div className="space-y-3">
              {kit.outreach.map((d) => (
                <div key={d.medium} className="rounded-lg border border-slate-200 p-3">
                  <div className="mb-2 flex items-center justify-between">
                    <span className="text-xs font-semibold text-slate-700">{mediumLabel(d.medium)}</span>
                    <Button
                      size="sm"
                      variant="ghost"
                      icon={<Icon name="check" className="h-3.5 w-3.5" />}
                      onClick={() => copy(d.body, d.medium)}
                    >
                      {copied === d.medium ? "Copied" : "Copy"}
                    </Button>
                  </div>
                  {d.subject && (
                    <p className="mb-1 text-xs text-slate-500">
                      <span className="font-medium">Subject:</span> {d.subject}
                    </p>
                  )}
                  <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-slate-50 p-2 text-xs text-slate-600">
                    {d.body}
                  </pre>
                </div>
              ))}
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
