import { useState } from "react";
import { api, auth } from "../api";
import Button from "../components/Button";
import { useToast } from "../components/Toast";

type Mode = "login" | "register";

export default function Login() {
  const [mode, setMode] = useState<Mode>("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const { notify } = useToast();

  const submit = async () => {
    const name = username.trim();
    if (!name) {
      notify("Enter your username", "error");
      return;
    }
    if (!password) {
      notify("Enter your password", "error");
      return;
    }
    if (mode === "register") {
      if (password.length < 8) {
        notify("Password must be at least 8 characters", "error");
        return;
      }
      if (password !== confirm) {
        notify("Passwords do not match", "error");
        return;
      }
    }
    setBusy(true);
    try {
      const { token } =
        mode === "login"
          ? await api.login(name, password)
          : await api.register(name, password);
      auth.setToken(token);
      window.location.reload();
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
            {mode === "login" ? "Log in to continue" : "Create your account"}
          </p>
        </div>

        <div className="space-y-4">
          {mode === "register" && (
            <p className="rounded-lg bg-indigo-50 p-3 text-xs leading-relaxed text-indigo-800">
              Each account has its own isolated data (leads, profile, settings,
              applications). Create an account to get started.
            </p>
          )}
          <div>
            <label className={label} htmlFor="login-username">
              Username
            </label>
            <input
              id="login-username"
              type="text"
              autoComplete="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              autoFocus
              className={input}
            />
          </div>
          <div>
            <label className={label} htmlFor="login-password">
              Password
            </label>
            <input
              id="login-password"
              type="password"
              autoComplete={mode === "login" ? "current-password" : "new-password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              className={input}
            />
          </div>
          {mode === "register" && (
            <div>
              <label className={label} htmlFor="login-confirm">
                Confirm password
              </label>
              <input
                id="login-confirm"
                type="password"
                autoComplete="new-password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && submit()}
                className={input}
              />
            </div>
          )}
          <Button className="w-full" loading={busy} onClick={submit}>
            {mode === "register" ? "Create account" : "Log in"}
          </Button>
          <p className="text-center text-xs text-slate-500">
            {mode === "login" ? (
              <>
                New here?{" "}
                <button
                  type="button"
                  onClick={() => {
                    setMode("register");
                    setConfirm("");
                  }}
                  className="font-medium text-indigo-600 hover:underline"
                >
                  Create an account
                </button>
              </>
            ) : (
              <>
                Already have an account?{" "}
                <button
                  type="button"
                  onClick={() => setMode("login")}
                  className="font-medium text-indigo-600 hover:underline"
                >
                  Log in
                </button>
              </>
            )}
          </p>
        </div>
      </div>
    </div>
  );
}
