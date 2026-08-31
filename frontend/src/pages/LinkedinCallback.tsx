import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { api } from "../api";
import Spinner from "../components/Spinner";

type Phase = "loading" | "ok" | "error";

export default function LinkedinCallback() {
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const [phase, setPhase] = useState<Phase>("loading");
  const [message, setMessage] = useState("");

  const code = useMemo(() => params.get("code") ?? "", [params]);
  const state = useMemo(() => params.get("state") ?? "", [params]);
  const error = useMemo(() => params.get("error") ?? "", [params]);
  const redirectUri = useMemo(
    () => `${window.location.origin}/linkedin/callback`,
    [],
  );

  useEffect(() => {
    let cancelled = false;

    (async () => {
      if (error) {
        setMessage(`LinkedIn returned an error: ${error}. Close this tab and try again.`);
        setPhase("error");
        return;
      }
      if (!code) {
        setMessage("No authorization code received from LinkedIn.");
        setPhase("error");
        return;
      }
      try {
        await api.linkedinCallback({ code, state, redirect_uri: redirectUri });
        if (cancelled) return;
        setMessage("LinkedIn connected. Taking you back to Settings...");
        setPhase("ok");
        setTimeout(() => navigate("/settings", { replace: true }), 800);
      } catch (e) {
        if (cancelled) return;
        setMessage(e instanceof Error ? e.message : "Failed to connect LinkedIn.");
        setPhase("error");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [code, state, error, redirectUri, navigate]);

  return (
    <div className="grid min-h-screen place-items-center bg-slate-50 p-6">
      <div className="w-full max-w-sm rounded-xl border border-slate-200 bg-white p-6 text-center shadow-sm">
        {phase === "loading" && (
          <>
            <Spinner className="mx-auto h-8 w-8" />
            <p className="mt-4 text-sm text-slate-600">Connecting your LinkedIn account...</p>
          </>
        )}
        {phase === "ok" && (
          <>
            <div className="mx-auto grid h-12 w-12 place-items-center rounded-full bg-emerald-100 text-emerald-600">
              <svg viewBox="0 0 24 24" className="h-6 w-6" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M20 6 9 17l-5-5" />
              </svg>
            </div>
            <p className="mt-4 text-sm font-medium text-slate-900">{message}</p>
          </>
        )}
        {phase === "error" && (
          <>
            <div className="mx-auto grid h-12 w-12 place-items-center rounded-full bg-rose-100 text-rose-600">
              <svg viewBox="0 0 24 24" className="h-6 w-6" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </div>
            <p className="mt-4 text-sm text-slate-700">{message}</p>
            <button
              onClick={() => navigate("/settings")}
              className="mt-4 rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-slate-700"
            >
              Back to Settings
            </button>
          </>
        )}
      </div>
    </div>
  );
}
