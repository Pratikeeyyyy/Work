import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { APPLICATION_STATUSES, type Application } from "../types";
import { timeAgo } from "../lib/format";
import { Badge, displayLabel, statusTone } from "../components/Badge";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Modal from "../components/Modal";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

const STAGES: { status: string; label: string; tone: string }[] = [
  { status: "saved", label: "Saved", tone: "text-slate-600" },
  { status: "applied", label: "Applied", tone: "text-sky-600" },
  { status: "replied", label: "Replied", tone: "text-violet-600" },
  { status: "interviewed", label: "Interviewed", tone: "text-indigo-600" },
  { status: "offered", label: "Offered", tone: "text-amber-600" },
  { status: "hired", label: "Hired", tone: "text-emerald-600" },
];

const REJECTED = ["rejected", "closed"];

export default function Applications() {
  const [apps, setApps] = useState<Application[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<Application | null>(null);
  const { notify } = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setApps(await api.listApplications());
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to load applications", "error");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const changeStatus = async (app: Application, status: string) => {
    try {
      await api.updateApplication(app.id, { status, follow_up: false });
      notify(`Marked as ${status}`);
      void load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Update failed", "error");
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

  if (loading) {
    return (
      <div className="grid place-items-center py-24">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div>
        <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Applications</h1>
        <p className="text-sm text-slate-500">
          Track every job/gig from applied through to hired, with follow-up nudges.
        </p>
      </div>

      {apps.length === 0 ? (
        <EmptyState
          icon="activity"
          title="No applications tracked"
          hint="Open a lead, generate an outreach draft, then track it as an application here when you apply."
        />
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
          {apps.map((app) => {
            const adv = app.status === "applied" || app.status === "replied" || app.status === "interviewed";
            return (
              <div
                key={app.id}
                className="flex flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-sm"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      {app.lead_source && <Badge tone={statusTone(app.lead_source)}>{displayLabel(app.lead_source)}</Badge>}
                      <Badge tone={statusTone(app.status)}>{displayLabel(app.status)}</Badge>
                      {adv && app.last_follow_up && app.last_follow_up < app.applied_at! && (
                        <span className="text-xs font-medium text-amber-600">Needs follow-up</span>
                      )}
                    </div>
                    <h3 className="mt-1.5 font-medium text-slate-900">{app.lead_title ?? "Untitled"}</h3>
                    <p className="text-sm text-slate-500">
                      {(app.company ?? "—")}
                      {app.contact ? ` · ${app.contact}` : ""}
                    </p>
                  </div>
                  {app.lead_url?.startsWith("http") && (
                    <a
                      href={app.lead_url}
                      target="_blank"
                      rel="noreferrer noopener"
                      className="shrink-0 rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-100 hover:text-indigo-600"
                      aria-label="Open listing"
                    >
                      <Icon name="external" className="h-4 w-4" />
                    </a>
                  )}
                </div>

                <div className="mt-4 flex items-center gap-1" aria-label="Pipeline stage">
                  {STAGES.map((s, i) => {
                    const currentIndex = STAGES.findIndex((x) => x.status === app.status);
                    const done = i <= currentIndex;
                    const active = i === currentIndex;
                    return (
                      <button
                        key={s.status}
                        onClick={() => !REJECTED.includes(app.status) && changeStatus(app, s.status)}
                        title={`Set status: ${s.label}`}
                        className={`flex flex-1 items-center gap-1 text-xs transition-colors ${done ? s.tone : "text-slate-300"}`}
                      >
                        <span
                          className={`grid h-5 w-5 place-items-center rounded-full border ${
                            active ? "border-current bg-current text-white" : "border-current"
                          }`}
                        >
                          {done && !active && <Icon name="check" className="h-3 w-3" />}
                          {active && <span className="h-1.5 w-1.5 rounded-full bg-white" />}
                        </span>
                        <span className={`hidden sm:inline ${active ? "font-semibold" : ""}`}>{s.label}</span>
                        {i < STAGES.length - 1 && <span className="h-px flex-1 bg-current opacity-30" />}
                      </button>
                    );
                  })}
                </div>

                <div className="mt-3 flex flex-wrap items-center gap-1.5">
                  {REJECTED.map((s) => (
                    <button
                      key={s}
                      onClick={() => changeStatus(app, s)}
                      className="rounded-full border border-slate-200 px-2 py-0.5 text-[11px] text-slate-500 transition-colors hover:border-rose-200 hover:text-rose-600"
                    >
                      Mark {s}
                    </button>
                  ))}
                </div>

                <div className="mt-3 space-y-1 border-t border-slate-100 pt-3 text-xs text-slate-500">
                  <div className="flex flex-wrap gap-x-4 gap-y-1">
                    {app.applied_at && <span>Applied {timeAgo(app.applied_at)}</span>}
                    {app.interviewed_at && <span>Interviewed {timeAgo(app.interviewed_at)}</span>}
                    {app.follow_up_count > 0 && <span>{app.follow_up_count} follow-ups</span>}
                  </div>
                  {app.next_scheduled && <span>Next: {app.next_scheduled}</span>}
                  {app.notes && <p className="text-slate-600">“{app.notes}”</p>}
                </div>

                <div className="mt-3 flex gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<Icon name="send" className="h-4 w-4" />}
                    onClick={() => followUp(app)}
                  >
                    Log follow-up
                  </Button>
                  <Button variant="secondary" size="sm" onClick={() => setEditing(app)}>
                    Edit
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {editing && (
        <EditApplicationModal
          app={editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            void load();
          }}
        />
      )}
    </div>
  );
}

function EditApplicationModal({
  app,
  onClose,
  onSaved,
}: {
  app: Application;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [status, setStatus] = useState(app.status);
  const [company, setCompany] = useState(app.company ?? "");
  const [contact, setContact] = useState(app.contact ?? "");
  const [next, setNext] = useState(app.next_scheduled ?? "");
  const [notes, setNotes] = useState(app.notes ?? "");
  const [saving, setSaving] = useState(false);
  const { notify } = useToast();

  const save = async () => {
    setSaving(true);
    try {
      await api.updateApplication(app.id, {
        status,
        company: company || null,
        contact: contact || null,
        next_scheduled: next || null,
        notes: notes || null,
        follow_up: false,
      });
      notify("Application updated");
      onSaved();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save", "error");
    } finally {
      setSaving(false);
    }
  };

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <Modal
      open
      title="Edit application"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={saving} onClick={save}>
            Save
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label className={label} htmlFor="app-status">
            Status
          </label>
          <select
            id="app-status"
            className={input}
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            {APPLICATION_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="app-company">
              Company
            </label>
            <input id="app-company" className={input} value={company} onChange={(e) => setCompany(e.target.value)} />
          </div>
          <div>
            <label className={label} htmlFor="app-contact">
              Contact
            </label>
            <input id="app-contact" className={input} value={contact} onChange={(e) => setContact(e.target.value)} />
          </div>
        </div>
        <div>
          <label className={label} htmlFor="app-next">
            Next follow-up (when)
          </label>
          <input id="app-next" className={input} value={next} onChange={(e) => setNext(e.target.value)} placeholder="e.g. Wed after 10am" />
        </div>
        <div>
          <label className={label} htmlFor="app-notes">
            Notes
          </label>
          <textarea id="app-notes" rows={3} className={input} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </div>
      </div>
    </Modal>
  );
}
