import { useEffect, useState } from "react";
import { api } from "../api";
import { SOURCES } from "../types";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import { useToast } from "../components/Toast";

const ALL_SOURCES = SOURCES.filter((s) => s !== "manual");

export default function Settings() {
  const [keywordsText, setKeywordsText] = useState("");
  const [sources, setSources] = useState<string[]>([...ALL_SOURCES]);
  const [savingKeywords, setSavingKeywords] = useState(false);
  const [savingSources, setSavingSources] = useState(false);
  const [loading, setLoading] = useState(true);
  const { notify } = useToast();

  useEffect(() => {
    void (async () => {
      try {
        const [k, s] = await Promise.all([api.getKeywords(), api.getSources()]);
        setKeywordsText(k.keywords.join(", "));
        setSources(s.keywords.filter((x) => x && x !== "manual"));
      } catch (e) {
        notify(e instanceof Error ? e.message : "Failed to load settings", "error");
      } finally {
        setLoading(false);
      }
    })();
  }, [notify]);

  const parseKeywords = () =>
    keywordsText
      .split(",")
      .map((k) => k.trim())
      .filter(Boolean);

  const saveKeywords = async () => {
    setSavingKeywords(true);
    try {
      await api.saveKeywords(parseKeywords());
      notify("Keywords saved");
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save keywords", "error");
    } finally {
      setSavingKeywords(false);
    }
  };

  const toggleSource = (source: string) => {
    setSources((prev) =>
      prev.includes(source) ? prev.filter((x) => x !== source) : [...prev, source],
    );
  };

  const saveSources = async () => {
    setSavingSources(true);
    try {
      await api.saveSources(sources);
      notify("Sources saved");
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save sources", "error");
    } finally {
      setSavingSources(false);
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
    <div className="max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Settings</h1>
        <p className="text-sm text-slate-500">Scraping keywords and enabled sources.</p>
      </div>

      <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-slate-900">Keywords</h2>
            <p className="text-sm text-slate-500">Comma-separated terms used to find matching jobs.</p>
          </div>
          <Button size="sm" loading={savingKeywords} onClick={saveKeywords}>
            Save
          </Button>
        </div>
        <textarea
          value={keywordsText}
          onChange={(e) => setKeywordsText(e.target.value)}
          rows={4}
          className="mt-4 w-full rounded-lg border border-slate-300 p-3 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20"
          placeholder="react, rust, blockchain, python"
        />
        <p className="mt-2 text-xs text-slate-500">{parseKeywords().length} keywords</p>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-slate-900">Sources</h2>
            <p className="text-sm text-slate-500">Which marketplaces the scraper queries.</p>
          </div>
          <Button size="sm" loading={savingSources} onClick={saveSources}>
            Save
          </Button>
        </div>
        <div className="mt-4 space-y-3">
          {ALL_SOURCES.map((source) => (
            <label
              key={source}
              className="flex cursor-pointer items-center gap-3 rounded-lg border border-slate-200 px-4 py-3 transition-colors hover:bg-slate-50 has-checked:border-indigo-500 has-checked:bg-indigo-50/50"
            >
              <input
                type="checkbox"
                checked={sources.includes(source)}
                onChange={() => toggleSource(source)}
                className="h-4 w-4 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
              />
              <span className="text-sm font-medium text-slate-700">{source}</span>
            </label>
          ))}
        </div>
      </section>
    </div>
  );
}