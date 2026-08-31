import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { Stats } from "../types";
import StatCard from "../components/StatCard";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Button from "../components/Button";
import { Badge, statusTone } from "../components/Badge";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

export default function Dashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);
  const [scraping, setScraping] = useState(false);
  const { notify } = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setStats(await api.stats());
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to load stats", "error");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const runScrape = async () => {
    setScraping(true);
    try {
      const res = await api.scrape();
      notify(`Scraped sources — ${res.inserted} new leads (${res.total_found} found)`);
      if (res.errors.length) {
        res.errors.slice(0, 2).forEach((err) => notify(err, "info"));
      }
      await load();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Scrape failed", "error");
    } finally {
      setScraping(false);
    }
  };

  if (loading && !stats) {
    return (
      <div className="grid place-items-center py-24">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  const s = stats ?? {
    total_leads: 0,
    new_leads: 0,
    applied_leads: 0,
    won_leads: 0,
    total_clients: 0,
    active_clients: 0,
    total_contracts: 0,
    by_source: [],
    top_technologies: [],
  };

  const maxSource = Math.max(1, ...s.by_source.map((x) => x.count));
  const maxTech = Math.max(1, ...s.top_technologies.map((x) => x.count));

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Dashboard</h1>
          <p className="text-sm text-slate-500">Pipeline overview for your freelance leads.</p>
        </div>
        <Button loading={scraping} icon={<Icon name="refresh" className="h-4 w-4" />} onClick={runScrape}>
          Run scrape now
        </Button>
      </div>

      <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:gap-4">
        <StatCard label="Total leads" value={s.total_leads} />
        <StatCard label="New" value={s.new_leads} />
        <StatCard label="Applied" value={s.applied_leads} />
        <StatCard label="Won" value={s.won_leads} />
        <StatCard label="Clients" value={s.total_clients} />
        <StatCard label="Active clients" value={s.active_clients} />
        <StatCard label="Contracts" value={s.total_contracts} />
        <StatCard label="Sources" value={s.by_source.length} />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-slate-900">Leads by source</h2>
          {s.by_source.length === 0 ? (
            <p className="mt-3 text-sm text-slate-500">No leads yet — run a scrape to populate the pipeline.</p>
          ) : (
            <ul className="mt-4 space-y-3">
              {s.by_source.map((b) => (
                <li key={b.source}>
                  <div className="mb-1 flex items-center justify-between text-sm">
                    <Badge tone={statusTone(b.source)}>{b.source}</Badge>
                    <span className="font-medium text-slate-600">{b.count}</span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-slate-100">
                    <div
                      className="h-full rounded-full bg-indigo-500"
                      style={{ width: `${(b.count / maxSource) * 100}%` }}
                    />
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-slate-900">Top technologies</h2>
          {s.top_technologies.length === 0 ? (
            <p className="mt-3 text-sm text-slate-500">
              Technologies are tagged when leads are scraped or added.
            </p>
          ) : (
            <ul className="mt-4 space-y-3">
              {s.top_technologies.map((t) => (
                <li key={t.tech}>
                  <div className="mb-1 flex items-center justify-between text-sm">
                    <span className="text-slate-700">{t.tech}</span>
                    <span className="font-medium text-slate-600">{t.count}</span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-slate-100">
                    <div
                      className="h-full rounded-full bg-emerald-500"
                      style={{ width: `${(t.count / maxTech) * 100}%` }}
                    />
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      {s.total_leads === 0 && (
        <EmptyState
          icon="leads"
          title="Your pipeline is empty"
          hint="Configure keywords in Settings, then run a scrape to pull matching jobs from Upwork, Freelancer and Fiverr."
        />
      )}
    </div>
  );
}