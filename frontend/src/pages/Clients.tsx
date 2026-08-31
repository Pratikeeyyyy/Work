import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { CLIENT_STATUSES, SOURCES, type Client, type NewClient } from "../types";
import { timeAgo } from "../lib/format";
import { Badge, displayLabel, statusTone } from "../components/Badge";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Modal from "../components/Modal";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

const emptyClient: NewClient = {
  lead_id: null,
  name: "",
  email: null,
  company: null,
  country: null,
  website: null,
  whatsapp: null,
  source: null,
  linkedin: null,
  past_work: null,
  preferences: null,
};

export default function Clients() {
  const [clients, setClients] = useState<Client[]>([]);
  const [statusFilter, setStatusFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const [addOpen, setAddOpen] = useState(false);
  const [editClient, setEditClient] = useState<Client | null>(null);
  const { notify } = useToast();

  const load = useCallback(
    async (status?: string) => {
      setLoading(true);
      try {
        setClients(await api.listClients(status || undefined));
      } catch (e) {
        notify(e instanceof Error ? e.message : "Failed to load clients", "error");
      } finally {
        setLoading(false);
      }
    },
    [notify],
  );

  useEffect(() => {
    void load(statusFilter);
  }, [load, statusFilter]);

  const onDelete = async (client: Client) => {
    if (!window.confirm(`Delete client "${client.name}"?`)) return;
    try {
      await api.deleteClient(client.id);
      notify("Client deleted");
      void load(statusFilter);
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
          <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Clients</h1>
          <p className="text-sm text-slate-500">People you converted from leads.</p>
        </div>
        <div className="flex gap-2">
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            aria-label="Filter by client status"
          >
            <option value="">All statuses</option>
            {CLIENT_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
          <Button size="sm" icon={<Icon name="userPlus" className="h-4 w-4" />} onClick={() => setAddOpen(true)}>
            Add client
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="grid place-items-center py-24">
          <Spinner className="h-8 w-8" />
        </div>
      ) : clients.length === 0 ? (
        <EmptyState
          icon="clients"
          title="No clients yet"
          hint="Convert a lead to a client from the Leads page, or add one manually."
        />
      ) : (
        <div className="overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm">
          <div className="overflow-x-auto">
            <table className="min-w-[860px] w-full text-left text-sm">
              <thead className="border-b border-slate-200 bg-slate-50 text-xs uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-3 py-3 font-medium">Company</th>
                  <th className="hidden px-3 py-3 font-medium md:table-cell">Country</th>
                  <th className="hidden px-3 py-3 font-medium lg:table-cell">Email</th>
                  <th className="hidden px-3 py-3 font-medium xl:table-cell">Source</th>
                  <th className="px-3 py-3 font-medium">Status</th>
                  <th className="hidden px-3 py-3 font-medium xl:table-cell">Added</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {clients.map((client) => (
                  <tr key={client.id} className="transition-colors hover:bg-slate-50">
                    <td className="px-4 py-3">
                      <p className="font-medium text-slate-900">{client.name}</p>
                      {client.country && (
                        <p className="text-xs text-slate-500 md:hidden">{client.country}</p>
                      )}
                    </td>
                    <td className="px-3 py-3 text-slate-600">{client.company ?? "—"}</td>
                    <td className="hidden px-3 py-3 text-slate-600 md:table-cell">
                      {client.country ?? "—"}
                    </td>
                    <td className="hidden max-w-[220px] truncate px-3 py-3 text-slate-600 lg:table-cell">
                      {client.email ?? "—"}
                    </td>
                    <td className="hidden px-3 py-3 xl:table-cell">
                      {client.source ? (
                        <Badge tone={statusTone(client.source)}>{displayLabel(client.source)}</Badge>
                      ) : (
                        "—"
                      )}
                    </td>
                    <td className="px-3 py-3">
                      <Badge tone={statusTone(client.status)}>{client.status}</Badge>
                    </td>
                    <td className="hidden px-3 py-3 text-slate-500 xl:table-cell">
                      {timeAgo(client.created_at)}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-1">
                        <button className={iconBtnClass} title="Edit" onClick={() => setEditClient(client)}>
                          <Icon name="edit" className="h-4 w-4" />
                        </button>
                        <button className={iconBtnClass} title="Delete" onClick={() => onDelete(client)}>
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

      <ClientFormModal
        open={addOpen}
        title="Add client"
        initial={emptyClient}
        onClose={() => setAddOpen(false)}
        onSaved={() => {
          setAddOpen(false);
          void load(statusFilter);
        }}
      />

      <ClientFormModal
        open={editClient !== null}
        title="Edit client"
        initial={editClient ? toNewClient(editClient) : emptyClient}
        mode={editClient ? editClient : null}
        onClose={() => setEditClient(null)}
        onSaved={(client) => {
          void (async () => {
            if (editClient) {
              try {
                await api.updateClient({ ...editClient, ...client });
                notify("Client updated");
              } catch (e) {
                notify(e instanceof Error ? e.message : "Update failed", "error");
              }
            }
          })();
          setEditClient(null);
          void load(statusFilter);
        }}
      />
    </div>
  );
}

function toNewClient(c: Client): NewClient {
  return {
    lead_id: c.lead_id,
    name: c.name,
    email: c.email,
    company: c.company,
    country: c.country,
    website: c.website,
    whatsapp: c.whatsapp,
    source: c.source,
    linkedin: c.linkedin,
    past_work: c.past_work,
    preferences: c.preferences,
  };
}

function ClientFormModal({
  open,
  title,
  initial,
  mode,
  onClose,
  onSaved,
}: {
  open: boolean;
  title: string;
  initial: NewClient;
  mode?: Client | null;
  onClose: () => void;
  onSaved: (client: NewClient) => void;
}) {
  const [form, setForm] = useState<NewClient>(initial);
  const [saving, setSaving] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    if (open) setForm(initial);
  }, [open, initial]);

  const set = <K extends keyof NewClient>(key: K, value: NewClient[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const submit = async () => {
    if (!form.name.trim()) {
      notify("Name is required", "error");
      return;
    }
    setSaving(true);
    try {
      if (mode) {
        await api.updateClient({ ...mode, ...form });
        onSaved({ ...form });
      } else {
        await api.addClient({ ...form });
        onSaved({ ...form });
      }
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save client", "error");
    } finally {
      setSaving(false);
    }
  };

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <Modal
      open={open}
      title={title}
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={saving} onClick={submit}>
            {mode ? "Save changes" : "Add client"}
          </Button>
        </>
      }
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <div>
          <label className={label} htmlFor="client-name">
            Name *
          </label>
          <input
            id="client-name"
            className={input}
            value={form.name}
            onChange={(e) => set("name", e.target.value)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-company">
            Company
          </label>
          <input
            id="client-company"
            className={input}
            value={form.company ?? ""}
            onChange={(e) => set("company", e.target.value || null)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-email">
            Email
          </label>
          <input
            id="client-email"
            type="email"
            className={input}
            value={form.email ?? ""}
            onChange={(e) => set("email", e.target.value || null)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-country">
            Country
          </label>
          <input
            id="client-country"
            className={input}
            value={form.country ?? ""}
            onChange={(e) => set("country", e.target.value || null)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-website">
            Website
          </label>
          <input
            id="client-website"
            className={input}
            value={form.website ?? ""}
            onChange={(e) => set("website", e.target.value || null)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-whatsapp">
            WhatsApp
          </label>
          <input
            id="client-whatsapp"
            className={input}
            value={form.whatsapp ?? ""}
            onChange={(e) => set("whatsapp", e.target.value || null)}
          />
        </div>
        <div>
          <label className={label} htmlFor="client-source">
            Source
          </label>
          <select
            id="client-source"
            className={input}
            value={form.source ?? ""}
            onChange={(e) => set("source", e.target.value || null)}
          >
            <option value="">—</option>
            {SOURCES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
        <div className="sm:col-span-2">
          <label className={label} htmlFor="client-prefs">
            Preferences
          </label>
          <input
            id="client-prefs"
            className={input}
            value={form.preferences ?? ""}
            onChange={(e) => set("preferences", e.target.value || null)}
            placeholder="Budget, deadlines, communication style…"
          />
        </div>
      </div>
    </Modal>
  );
}