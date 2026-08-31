import { useEffect, useState } from "react";
import { api, auth } from "../api";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import { useToast } from "../components/Toast";

type Mode = "checking" | "login" | "setup";

export default function Login() {
  const [mode, setMode] = useState<Mode>("checking");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    let cancelled = false;
    void api
      .authStatus()
      .then((s) => {
        if (cancelled) return;
        setMode(s.hasPassword ? "login" : "setup");
      })
      .catch(() => {
        if (cancelled) return;
        setMode("login");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const submit = async () => {
    setBusy(true);
    try {
      if (mode === "login") {
        if (!password) {
          notify("Enter your password", "error");
          return;
        }
        const { token } = await api.login(password);
        auth.setToken(token);
        window.location.reload();
      } else {
        if (password.length < 8) {
          notify("Password must be at least 8 characters", "error");
          return;
        }
        if (password !== confirm) {
          notify("Passwords do not match", "error");
          return;
        }
        const { token } = await api.setup(password);
        auth.setToken(token);
        window.location.reload();
      }
    } catch (e) {
      notify(e instanceof Error ? e.message : "Something went wrong", "error");
    } finally {
      setBusy(false);
    }
  };

  const input =
    "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <div className="grid min-h-screen place-items-center bg-slate-50 p-6">
      <div className="w-full max-w-sm rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <div className="mb-5 text-center">
          <h1 className="text-lg font-bold text-slate-900">LeadGen</h1>
          <p className="mt-1 text-sm text-slate-500">
            {mode === "checking"
              ? "Checking...  "
              : mode === "setup"
                ? "Create your login password"
                : "Log in to continue"}
          </p>
        </div>

        {mode === "checking" ? (
          <div className="grid place-items-center py-8">
            <Spinner className="h-6 w-6" />
          </div>
        ) : (
          <div className="space-y-4">
            {mode === "setup" && (
              <p className="rounded-lg bg-indigo-50 p-3 text-xs leading-relaxed text-indigo-800">
                First run. Set a password to secure this instance. It is stored hashed
                (PBKDF2) and never shared. For deployed instances, prefer setting the
                <code className="rounded bg-slate-200/60 px-1"> APP_PASSWORD </code>
                environment variable instead.
              </p>
            )}
            <div>
              <label className={label} htmlFor="login-password">
                {mode === "setup" ? "New password" : "Password"}
              </label>
              <input
                id="login-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && submit()}
                autoFocus
                className={input}
              />
            </div>
            {mode === "setup" && (
              <div>
                <label className={label} htmlFor="login-confirm">
                  Confirm password
                </label>
                <input
                  id="login-confirm"
                  type="password"
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && submit()}
                  className={input}
                />
              </div>
            )}
            <Button className="w-full" loading={busy} onClick={submit}>
              {mode === "setup" ? "Create password" : "Log in"}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
