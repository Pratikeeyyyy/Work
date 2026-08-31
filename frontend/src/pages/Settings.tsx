import { useEffect, useState } from "react";
import { api } from "../api";
import { SOURCES, type Profile } from "../types";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

const ALL_SOURCES = SOURCES.filter((s) => s !== "manual");

const initialProfile: Profile = {
  name: null,
  title: null,
  email: null,
  location: null,
  rate: null,
  skills: [],
  experience: null,
  availability: null,
  bio: null,
  portfolio: null,
  linkedin: null,
  github: null,
};

export default function Settings() {
  const [keywordsText, setKeywordsText] = useState("");
  const [sources, setSources] = useState<string[]>([...ALL_SOURCES]);
  const [savingKeywords, setSavingKeywords] = useState(false);
  const [savingSources, setSavingSources] = useState(false);
  const [loading, setLoading] = useState(true);
  const { notify } = useToast();

  // Profile state
  const [profile, setProfile] = useState<Profile>(initialProfile);
  const [savingProfile, setSavingProfile] = useState(false);

  // LinkedIn state
  const [liClientId, setLiClientId] = useState("");
  const [liSecret, setLiSecret] = useState("");
  const [liRedirect, setLiRedirect] = useState("http://localhost:5173/linkedin/callback");
  const [liClientSecretSet, setLiClientSecretSet] = useState(false);
  const [savingLinkedin, setSavingLinkedin] = useState(false);
  const [liConnected, setLiConnected] = useState(false);
  const [liMember, setLiMember] = useState("");

  useEffect(() => {
    void (async () => {
      try {
        const [k, s, p, ls, lstatus] = await Promise.all([
          api.getKeywords(),
          api.getSources(),
          api.getProfile(),
          api.getLinkedinSettings(),
          api.linkedinStatus(),
        ]);
        setKeywordsText(k.keywords.join(", "));
        setSources(s.keywords.filter((x) => x && x !== "manual"));
        setProfile(p);
        setLiClientId(ls.client_id);
        setLiClientSecretSet(ls.client_secret_set);
        setLiRedirect(ls.redirect_uri);
        setLiConnected(lstatus.connected);
        setLiMember(lstatus.member_name);
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

  const setProfileField = <K extends keyof Profile>(key: K, value: Profile[K]) =>
    setProfile((prev) => ({ ...prev, [key]: value }));

  const saveProfile = async () => {
    setSavingProfile(true);
    try {
      await api.saveProfile(profile);
      notify("Profile saved — use Rescore on the Leads page to re-rank leads");
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save profile", "error");
    } finally {
      setSavingProfile(false);
    }
  };

  const saveLinkedinSettings = async () => {
    setSavingLinkedin(true);
    try {
      await api.saveLinkedinSettings({
        client_id: liClientId || undefined,
        client_secret: liSecret || undefined,
        redirect_uri: liRedirect || undefined,
      });
      notify("LinkedIn app settings saved");
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to save LinkedIn settings", "error");
    } finally {
      setSavingLinkedin(false);
    }
  };

  const openLinkedinAuth = async () => {
    try {
      const { url } = await api.linkedinAuthUrl(liRedirect);
      window.open(url, "_blank", "noopener,noreferrer,width=640,height=720");
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to start LinkedIn auth", "error");
    }
  };

  if (loading) {
    return (
      <div className="grid place-items-center py-24">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <div className="max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Settings</h1>
        <p className="text-sm text-slate-500">
          Profile for scoring &amp; outreach, scraping options, and LinkedIn connection.
        </p>
      </div>

      <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-slate-900">My profile</h2>
            <p className="text-sm text-slate-500">
              Used to score lead fit (higher = better match) and to personalise outreach drafts.
            </p>
          </div>
          <Button size="sm" loading={savingProfile} onClick={saveProfile}>
            Save
          </Button>
        </div>
        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="p-name">Name</label>
            <input id="p-name" className={input} value={profile.name ?? ""} onChange={(e) => setProfileField("name", e.target.value || null)} />
          </div>
          <div>
            <label className={label} htmlFor="p-title">Title</label>
            <input id="p-title" className={input} value={profile.title ?? ""} onChange={(e) => setProfileField("title", e.target.value || null)} placeholder="Full-Stack Developer" />
          </div>
          <div>
            <label className={label} htmlFor="p-email">Email</label>
            <input id="p-email" className={input} value={profile.email ?? ""} onChange={(e) => setProfileField("email", e.target.value || null)} />
          </div>
          <div>
            <label className={label} htmlFor="p-location">Location</label>
            <input id="p-location" className={input} value={profile.location ?? ""} onChange={(e) => setProfileField("location", e.target.value || null)} placeholder="Remote" />
          </div>
          <div>
            <label className={label} htmlFor="p-rate">Rate</label>
            <input id="p-rate" className={input} value={profile.rate ?? ""} onChange={(e) => setProfileField("rate", e.target.value || null)} placeholder="$50/hr" />
          </div>
          <div>
            <label className={label} htmlFor="p-exp">Experience</label>
            <input id="p-exp" className={input} value={profile.experience ?? ""} onChange={(e) => setProfileField("experience", e.target.value || null)} placeholder="5+ years" />
          </div>
          <div>
            <label className={label} htmlFor="p-avail">Availability</label>
            <input id="p-avail" className={input} value={profile.availability ?? ""} onChange={(e) => setProfileField("availability", e.target.value || null)} placeholder="Full-time" />
          </div>
          <div>
            <label className={label} htmlFor="p-portfolio">Portfolio</label>
            <input id="p-portfolio" className={input} value={profile.portfolio ?? ""} onChange={(e) => setProfileField("portfolio", e.target.value || null)} placeholder="https://…" />
          </div>
          <div>
            <label className={label} htmlFor="p-linkedin">LinkedIn</label>
            <input id="p-linkedin" className={input} value={profile.linkedin ?? ""} onChange={(e) => setProfileField("linkedin", e.target.value || null)} placeholder="linkedin.com/in/…" />
          </div>
          <div>
            <label className={label} htmlFor="p-github">GitHub</label>
            <input id="p-github" className={input} value={profile.github ?? ""} onChange={(e) => setProfileField("github", e.target.value || null)} placeholder="github.com/…" />
          </div>
        </div>
        <div className="mt-4">
          <label className={label} htmlFor="p-skills">Skills (comma-separated — drives fit scoring)</label>
          <input id="p-skills" className={input} value={profile.skills.join(", ")} onChange={(e) => setProfileField("skills", e.target.value.split(",").map((s) => s.trim()).filter(Boolean))} placeholder="react, rust, python, solidity" />
        </div>
        <div className="mt-4">
          <label className={label} htmlFor="p-bio">Short bio / pitch</label>
          <textarea id="p-bio" rows={3} className={input} value={profile.bio ?? ""} onChange={(e) => setProfileField("bio", e.target.value || null)} />
        </div>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-slate-900">LinkedIn connection</h2>
            <p className="text-sm text-slate-500">Official OAuth app (no password, no scraping risk).</p>
          </div>
          {liConnected ? (
            <span className="inline-flex items-center gap-1 rounded-full bg-emerald-100 px-2.5 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-inset ring-emerald-600/10">
              <Icon name="check" className="h-3.5 w-3.5" /> Connected{liMember ? ` · ${liMember}` : ""}
            </span>
          ) : (
            <span className="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs font-medium text-slate-500">Not connected</span>
          )}
        </div>
        <div className="mt-4 grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="li-client">Client ID</label>
            <input id="li-client" className={input} value={liClientId} onChange={(e) => setLiClientId(e.target.value)} />
          </div>
          <div>
            <label className={label} htmlFor="li-secret">Client Secret</label>
            <input id="li-secret" className={input} type="password" value={liSecret} onChange={(e) => setLiSecret(e.target.value)} placeholder={liClientSecretSet ? "•••••••• (leave blank to keep)" : ""} />
          </div>
        </div>
        <div className="mt-4">
          <label className={label} htmlFor="li-redirect">Redirect URI</label>
          <input id="li-redirect" className={input} value={liRedirect} onChange={(e) => setLiRedirect(e.target.value)} />
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <Button size="sm" loading={savingLinkedin} onClick={saveLinkedinSettings}>
            Save app settings
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={openLinkedinAuth}
            disabled={!liClientId}
            title={liClientId ? "Continue with LinkedIn" : "Save a Client ID first"}
          >
            Continue with LinkedIn
          </Button>
        </div>
        <p className="mt-3 text-xs text-slate-500">
          After authorizing, LinkedIn redirects you back with a code. Follow the steps in{" "}
          <code className="rounded bg-slate-100 px-1">SETUP.md</code>. This connects your account legally for
          job research and future outreach — it never uses your password.
        </p>
      </section>

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
            <p className="text-sm text-slate-500">Which sources the scraper queries.</p>
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
        <p className="mt-3 text-xs text-slate-500">
          Tip: Indeed is the reliable source for full-time/remote jobs. Upwork, Fiverr and Freelancer
          block automated scraping and may require manual import instead.
        </p>
      </section>
    </div>
  );
}