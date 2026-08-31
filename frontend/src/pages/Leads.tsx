import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { LEAD_STATUSES, SOURCES, type Lead, type NewLead, type OutreachDraft } from "../types";
import { joinTags, timeAgo } from "../lib/format";
import { Badge, displayLabel, statusTone } from "../components/Badge";
import ScorePill from "../components/ScorePill";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Modal from "../components/Modal";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

interface Filters {
  q: string;
  status: string;
  source: string;
}

const initialFilters: Filters = { q: "", status: "", source: "" };

const emptyLead: NewLead = {
  source: "manual",
  title: "",
  description: "",
  url: "",
  budget: null,
  budget_min: null,
  budget_max: null,
  currency: null,
  location: null,
  technologies: null,
  client_name: null,
  posted_date: null,
};

export default function Leads() {
  const [leads, setLeads] = useState<Lead[]>([]);
  const [filters, setFilters] = useState<Filters>(initialFilters);
  const [loading, setLoading] = useState(true);
  const [scraping, setScraping] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [notesLead, setNotesLead] = useState<Lead | null>(null);
  const [outreachLead, setOutreachLead] = useState<Lead | null>(null);
  const [scrapeOpen, setScrapeOpen] = useState(false);
  const { notify } = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setLeads(
        await api.listLeads({
          q: filters.q || undefined,
          status: filters.status || undefined,
          source: filters.source || undefined,
        }),
      );
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to load leads", "error");
    } finally {
      setLoading(false);
    }
  }, [filters, notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const runScrape = async () => {
    setScraping(true);
    try {
      const res = await api.scrape();
      notify(`Scrape done — ${res.inserted} new leads (${res.total_found} found)`);
      if (res.errors.length) res.errors.slice(0, 2).forEach((err) => notify(err, "info"));
      await load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Scrape failed", "error");
    } finally {
      setScraping(false);
    }
  };

  const onStatus = async (lead: Lead, status: string) => {
    try {
      await api.updateLeadStatus(lead.id, status);
      setLeads((prev) => prev.map((l) => (l.id === lead.id ? { ...l, status } : l)));
    } catch (e) {
      notify(e instanceof Error ? e.message : "Status update failed", "error");
    }
  };

  const onNotes = async () => {
    if (!notesLead) return;
    try {
      await api.updateLeadNotes(notesLead.id, notesLead.notes ?? "");
      notify("Notes saved");
      setNotesLead(null);
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save notes", "error");
    }
  };

  const onConvert = async (lead: Lead) => {
    try {
      await api.convertLead(lead.id);
      notify(`Converted "${lead.title}" to a client`);
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Conversion failed", "error");
    }
  };

  const onTrack = async (lead: Lead) => {
    try {
      await api.addApplication({ lead_id: lead.id, company: lead.client_name, notes: lead.notes });
      notify(`"${lead.title}" added to applications`);
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to track application", "error");
    }
  };

  const onDelete = async (lead: Lead) => {
    if (!window.confirm(`Delete lead "${lead.title}"?`)) return;
    try {
      await api.deleteLead(lead.id);
      notify("Lead deleted");
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Delete failed", "error");
    }
  };

  const iconBtnClass =
    "rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-100 hover:text-slate-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500";

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Leads</h1>
          <p className="text-sm text-slate-500">Matching jobs from your enabled marketplaces.</p>
        </div>
        <div className="flex gap-2">
          <Button variant="secondary" size="sm" icon={<Icon name="link" className="h-4 w-4" />} onClick={() => setImportOpen(true)}>
            Import URL
          </Button>
          <Button variant="secondary" size="sm" icon={<Icon name="refresh" className="h-4 w-4" />} onClick={() => setScrapeOpen(true)}>
            Scrape
          </Button>
          <Button size="sm" icon={<Icon name="plus" className="h-4 w-4" />} onClick={() => setAddOpen(true)}>
            Add lead
          </Button>
        </div>
      </div>

      <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
        <div className="relative flex-1">
          <span className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-slate-400">
            <Icon name="search" className="h-4 w-4" />
          </span>
          <input
            value={filters.q}
            onChange={(e) => setFilters((f) => ({ ...f, q: e.target.value }))}
            placeholder="Search title, tech, client…"
            className="w-full rounded-lg border border-slate-300 bg-white py-2 pl-9 pr-3 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
          />
        </div>
        <div className="flex gap-2">
          <select
            value={filters.status}
            onChange={(e) => setFilters((f) => ({ ...f, status: e.target.value }))}
            className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            aria-label="Filter by status"
          >
            <option value="">All statuses</option>
            {LEAD_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
          <select
            value={filters.source}
            onChange={(e) => setFilters((f) => ({ ...f, source: e.target.value }))}
            className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            aria-label="Filter by source"
          >
            <option value="">All sources</option>
            {SOURCES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
      </div>

      {loading ? (
        <div className="grid place-items-center py-24">
          <Spinner className="h-8 w-8" />
        </div>
      ) : leads.length === 0 ? (
        <EmptyState
          icon="leads"
          title={filters.q || filters.status || filters.source ? "No leads match your filters" : "No leads yet"}
          hint="Add a lead manually or run a scrape to fill the pipeline."
          action={
            <div className="flex justify-center gap-2">
              <Button variant="secondary" size="sm" onClick={() => setScrapeOpen(true)}>
                Run scrape
              </Button>
              <Button size="sm" onClick={() => setAddOpen(true)}>
                Add lead
              </Button>
            </div>
          }
        />
      ) : (
        <div className="overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm">
          <div className="overflow-x-auto">
            <table className="min-w-[900px] w-full text-left text-sm">
              <thead className="border-b border-slate-200 bg-slate-50 text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Title</th>
                  <th className="px-3 py-3 font-medium">Source</th>
                  <th className="px-3 py-3 font-medium">Score</th>
                  <th className="px-3 py-3 font-medium">Budget</th>
                  <th className="hidden px-3 py-3 font-medium md:table-cell">Tech</th>
                  <th className="hidden px-3 py-3 font-medium lg:table-cell">Client</th>
                  <th className="hidden px-3 py-3 font-medium xl:table-cell">Found</th>
                  <th className="px-3 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {leads.map((lead) => (
                  <tr key={lead.id} className="transition-colors hover:bg-slate-50">
                    <td className="max-w-[280px] px-4 py-3">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium text-slate-900">{lead.title}</p>
                        {lead.url.startsWith("http") && (
                          <a
                            href={lead.url}
                            target="_blank"
                            rel="noreferrer noopener"
                            className="shrink-0 text-slate-400 transition-colors hover:text-indigo-600"
                            aria-label="Open original listing"
                          >
                            <Icon name="external" className="h-3.5 w-3.5" />
                          </a>
                        )}
                      </div>
                      {lead.location && (
                        <p className="truncate text-xs text-slate-500">{lead.location}</p>
                      )}
                    </td>
                    <td className="px-3 py-3">
                      <Badge tone={statusTone(lead.source)}>{displayLabel(lead.source)}</Badge>
                    </td>
                    <td className="px-3 py-3">
                      <ScorePill score={lead.score} />
                    </td>
                    <td className="px-3 py-3 text-slate-700">{lead.budget ?? "—"}</td>
                    <td className="hidden px-3 py-3 md:table-cell">
                      <div className="flex max-w-[200px] flex-wrap gap-1">
                        {joinTags(lead.technologies)
                          .slice(0, 3)
                          .map((t) => (
                            <span key={t} className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-600">
                              {t}
                            </span>
                          ))}
                        {joinTags(lead.technologies).length > 3 && (
                          <span className="text-xs text-slate-400">
                            +{joinTags(lead.technologies).length - 3}
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="hidden max-w-[160px] truncate px-3 py-3 text-slate-600 lg:table-cell">
                      {lead.client_name ?? "—"}
                    </td>
                    <td className="hidden px-3 py-3 text-slate-500 xl:table-cell">
                      {timeAgo(lead.created_at)}
                    </td>
                    <td className="px-3 py-3">
                      <select
                        value={lead.status}
                        onChange={(e) => onStatus(lead, e.target.value)}
                        className={`rounded-md border-0 bg-transparent py-1 text-xs font-medium ring-1 ring-inset focus:outline-none focus:ring-2 focus:ring-indigo-500 ${
                          {
                            new: "text-sky-700 ring-sky-600/20",
                            shortlisted: "text-indigo-700 ring-indigo-600/20",
                            applied: "text-amber-700 ring-amber-600/20",
                            responded: "text-violet-700 ring-violet-600/20",
                            won: "text-emerald-700 ring-emerald-600/20",
                            lost: "text-rose-700 ring-rose-600/20",
                            archived: "text-slate-500 ring-slate-600/20",
                          }[lead.status]
                        }`}
                      >
                        {LEAD_STATUSES.map((s) => (
                          <option key={s} value={s} className="text-slate-700">
                            {s}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-1">
                        <button
                          className={iconBtnClass}
                          title="Generate outreach"
                          onClick={() => setOutreachLead(lead)}
                        >
                          <Icon name="send" className="h-4 w-4" />
                        </button>
                        {!["won", "lost", "archived"].includes(lead.status) && (
                          <button
                            className={iconBtnClass}
                            title="Track application"
                            onClick={() => onTrack(lead)}
                          >
                            <Icon name="activity" className="h-4 w-4" />
                          </button>
                        )}
                        <button
                          className={iconBtnClass}
                          title="Edit notes"
                          onClick={() => setNotesLead(lead)}
                        >
                          <Icon name="edit" className="h-4 w-4" />
                        </button>
                        {!["won", "lost", "archived"].includes(lead.status) && (
                          <button
                            className={iconBtnClass}
                            title="Convert to client"
                            onClick={() => onConvert(lead)}
                          >
                            <Icon name="userPlus" className="h-4 w-4" />
                          </button>
                        )}
                        <button className={iconBtnClass} title="Delete" onClick={() => onDelete(lead)}>
                          <Icon name="trash" className="h-4 w-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <ScrapeModal
        open={scrapeOpen}
        running={scraping}
        onClose={() => setScrapeOpen(false)}
        onRun={runScrape}
      />

      <ImportModal
        open={importOpen}
        onClose={() => setImportOpen(false)}
        onSaved={() => {
          setImportOpen(false);
          void load();
        }}
      />

      <OutreachModal lead={outreachLead} onClose={() => setOutreachLead(null)} />

      <AddLeadModal
        open={addOpen}
        onClose={() => setAddOpen(false)}
        onSaved={() => {
          setAddOpen(false);
          void load();
        }}
      />

      <Modal
        open={notesLead !== null}
        title="Lead notes"
        onClose={() => setNotesLead(null)}
        footer={
          <>
            <Button variant="secondary" onClick={() => setNotesLead(null)}>
              Cancel
            </Button>
            <Button onClick={onNotes}>Save notes</Button>
          </>
        }
      >
        <label className="block text-sm font-medium text-slate-700" htmlFor="lead-notes">
          Notes
        </label>
        <textarea
          id="lead-notes"
          rows={5}
          value={notesLead?.notes ?? ""}
          onChange={(e) =>
            setNotesLead((l) => (l ? { ...l, notes: e.target.value } : l))
          }
          className="mt-2 w-full rounded-lg border border-slate-300 p-3 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
          placeholder="Client preferences, next step, deadlines…"
        />
      </Modal>
    </div>
  );
}

function ScrapeModal({
  open,
  running,
  onClose,
  onRun,
}: {
  open: boolean;
  running: boolean;
  onClose: () => void;
  onRun: () => void;
}) {
  return (
    <Modal
      open={open}
      title="Run scrape"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={running}>
            Cancel
          </Button>
          <Button loading={running} icon={<Icon name="refresh" className="h-4 w-4" />} onClick={onRun}>
            Start scraping
          </Button>
        </>
      }
    >
      <p className="text-sm text-slate-600">
        Pulls new matching jobs from every enabled source using your configured keywords. New
        listings are deduplicated by URL, scored, and added to the pipeline.
      </p>
    </Modal>
  );
}

function AddLeadModal({
  open,
  onClose,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<NewLead>(emptyLead);
  const [saving, setSaving] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    if (open) setForm(emptyLead);
  }, [open]);

  const set = <K extends keyof NewLead>(key: K, value: NewLead[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const submit = async () => {
    if (!form.title.trim()) {
      notify("Title is required", "error");
      return;
    }
    setSaving(true);
    try {
      await api.addLead({
        ...form,
        title: form.title.trim(),
        url: form.url.trim(),
        technologies: form.technologies?.trim() ? form.technologies.trim() : null,
      });
      notify("Lead added");
      onSaved();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to add lead", "error");
    } finally {
      setSaving(false);
    }
  };

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <Modal
      open={open}
      title="Add lead"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={saving} onClick={submit}>
            Add lead
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label className={label} htmlFor="lead-title">
            Title *
          </label>
          <input
            id="lead-title"
            className={input}
            value={form.title}
            onChange={(e) => set("title", e.target.value)}
            placeholder="React dashboard for fintech startup"
          />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="lead-source">
              Source
            </label>
            <select
              id="lead-source"
              className={input}
              value={form.source}
              onChange={(e) => set("source", e.target.value)}
            >
              {SOURCES.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className={label} htmlFor="lead-url">
              URL
            </label>
            <input
              id="lead-url"
              className={input}
              value={form.url}
              onChange={(e) => set("url", e.target.value)}
              placeholder="https://…"
            />
          </div>
          <div>
            <label className={label} htmlFor="lead-budget">
              Budget
            </label>
            <input
              id="lead-budget"
              className={input}
              value={form.budget ?? ""}
              onChange={(e) => set("budget", e.target.value || null)}
              placeholder="$1,000 — $5,000"
            />
          </div>
          <div>
            <label className={label} htmlFor="lead-currency">
              Currency
            </label>
            <input
              id="lead-currency"
              className={input}
              value={form.currency ?? ""}
              onChange={(e) => set("currency", e.target.value || null)}
              placeholder="USD"
            />
          </div>
          <div>
            <label className={label} htmlFor="lead-location">
              Location
            </label>
            <input
              id="lead-location"
              className={input}
              value={form.location ?? ""}
              onChange={(e) => set("location", e.target.value || null)}
              placeholder="Remote"
            />
          </div>
          <div>
            <label className={label} htmlFor="lead-client">
              Client name
            </label>
            <input
              id="lead-client"
              className={input}
              value={form.client_name ?? ""}
              onChange={(e) => set("client_name", e.target.value || null)}
            />
          </div>
        </div>
        <div>
          <label className={label} htmlFor="lead-tech">
            Technologies
          </label>
          <input
            id="lead-tech"
            className={input}
            value={form.technologies ?? ""}
            onChange={(e) => set("technologies", e.target.value)}
            placeholder="react, typescript, api"
          />
        </div>
        <div>
          <label className={label} htmlFor="lead-desc">
            Description
          </label>
          <textarea
            id="lead-desc"
            rows={3}
            className={input}
            value={form.description}
            onChange={(e) => set("description", e.target.value)}
            placeholder="What does the client need?"
          />
        </div>
      </div>
    </Modal>
  );
}

function ImportModal({
  open,
  onClose,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [url, setUrl] = useState("");
  const [saving, setSaving] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    if (open) setUrl("");
  }, [open]);

  const submit = async () => {
    if (!url.trim()) {
      notify("Paste a job or gig URL first", "error");
      return;
    }
    setSaving(true);
    try {
      await api.importLeadUrl(url.trim());
      notify("Imported — add details and run scoring");
      onSaved();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Import failed", "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Import from URL"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={saving} onClick={submit}>
            Import
          </Button>
        </>
      }
    >
      <p className="mb-3 text-sm text-slate-600">
        Paste any job, gig or client link from Upwork, Indeed, LinkedIn, Fiverr or a Facebook group.
        The app saves it so you can score it against your profile and generate outreach.
      </p>
      <label className="block text-sm font-medium text-slate-700" htmlFor="import-url">
        URL
      </label>
      <input
        id="import-url"
        className="mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="https://www.indeed.com/viewjob?jk=…"
      />
    </Modal>
  );
}

function OutreachModal({
  lead,
  onClose,
}: {
  lead: Lead | null;
  onClose: () => void;
}) {
  const [drafts, setDrafts] = useState<OutreachDraft[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const { notify } = useToast();

  useEffect(() => {
    if (!lead) return;
    setDrafts(null);
    setLoading(true);
    void api
      .leadOutreach(lead.id)
      .then(setDrafts)
      .catch((e) => notify(e instanceof Error ? e.message : "Failed to generate outreach", "error"))
      .finally(() => setLoading(false));
  }, [lead, notify]);

  const copy = async (id: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(id);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      notify("Copy failed", "error");
    }
  };

  const mediumLabel = (m: string) =>
    ({ proposal: "Freelance proposal", linkedin_message: "LinkedIn message", email: "Email" })[m] ?? m;

  return (
    <Modal open={lead !== null} title="Generated outreach" onClose={onClose}>
      <div className="space-y-4">
        {loading && (
          <div className="grid place-items-center py-8">
            <Spinner className="h-6 w-6" />
          </div>
        )}
        {drafts?.map((d) => (
          <div key={d.medium} className="rounded-lg border border-slate-200 p-3">
            <div className="mb-2 flex items-center justify-between gap-2">
              <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">
                {mediumLabel(d.medium)}
              </span>
              <button
                onClick={() => copy(d.medium, d.body)}
                className="text-xs font-medium text-indigo-600 hover:text-indigo-800"
              >
                {copied === d.medium ? "Copied" : "Copy"}
              </button>
            </div>
            {d.subject && <p className="mb-1 text-sm font-medium text-slate-700">Subject: {d.subject}</p>}
            <pre className="whitespace-pre-wrap break-words rounded bg-slate-50 p-3 text-xs leading-relaxed text-slate-700">
              {d.body}
            </pre>
          </div>
        ))}
        {lead && !loading && !drafts && (
          <p className="text-sm text-slate-500">Could not generate outreach for this lead.</p>
        )}
        <p className="text-xs text-slate-500">
          Personalise these drafts and fill your profile (Settings → My profile) for better results.
        </p>
      </div>
    </Modal>
  );
}