import { useState, type FormEvent } from "react";
import { api, auth } from "../api";
import Button from "../components/Button";
import { useToast } from "../components/Toast";

type Mode = "login" | "register";
type Field = "username" | "password" | "confirm";
type FieldErrors = Partial<Record<Field, string>>;

function validUsername(name: string): string | undefined {
  if (!name) return "Username is required";
  if (name.length < 3 || name.length > 64) return "Username must be 3-64 characters";
  if (!/^[A-Za-z0-9._-]+$/.test(name))
    return "Usernames can only contain letters, digits, _ - .";
  return undefined;
}

export default function Login() {
  const [mode, setMode] = useState<Mode>("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [errors, setErrors] = useState<FieldErrors>({});
  const [busy, setBusy] = useState(false);
  const { notify } = useToast();

  const switchMode = (next: Mode) => {
    setMode(next);
    setConfirm("");
    setErrors({});
  };

  const fieldError = (field: Field): string | undefined => {
    if (field === "username") return validUsername(username.trim());
    if (field === "password") {
      if (!password) return "Password is required";
      if (mode === "register" && password.length < 8)
        return "Password must be at least 8 characters";
      if (mode === "register" && password.length > 128)
        return "Password must be at most 128 characters";
      return undefined;
    }
    if (mode === "register") {
      if (!confirm) return "Confirm your password";
      if (confirm !== password) return "Passwords do not match";
    }
    return undefined;
  };

  const invalidate = (field: Field) =>
    setErrors((prev) => ({ ...prev, [field]: fieldError(field) }));

  const clearError = (field: Field) =>
    setErrors((prev) => ({ ...prev, [field]: undefined }));

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const errs: FieldErrors = {
      username: fieldError("username"),
      password: fieldError("password"),
      confirm: fieldError("confirm"),
    };
    setErrors(errs);
    if (Object.values(errs).some(Boolean)) return;

    setBusy(true);
    try {
      const { token } =
        mode === "login"
          ? await api.login(username.trim(), password)
          : await api.register(username.trim(), password);
      auth.setToken(token);
      window.location.reload();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Something went wrong", "error");
    } finally {
      setBusy(false);
    }
  };

  const base =
    "mt-1 w-full rounded-lg border px-3 py-2 text-sm focus:outline-none focus:ring-2";
  const inputCls = (invalid: boolean) =>
    `${base} ${
      invalid
        ? "border-rose-300 focus:border-rose-500 focus:ring-rose-500/20"
        : "border-slate-300 focus:border-indigo-500 focus:ring-indigo-500/20"
    }`;
  const label = "block text-sm font-medium text-slate-700";
  const errorCls = "mt-1 text-xs text-rose-600";

  const fieldProps = (field: Field) => {
    const id = `login-${field}`;
    return {
      id,
      "aria-invalid": !!errors[field] || undefined,
      "aria-describedby": errors[field] ? `${id}-error` : undefined,
    };
  };

  return (
    <div className="grid min-h-screen place-items-center bg-slate-50 p-6">
      <div className="w-full max-w-sm rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <div className="mb-5 text-center">
          <h1 className="text-lg font-bold text-slate-900">LeadGen</h1>
          <p className="mt-1 text-sm text-slate-500">
            {mode === "login" ? "Log in to continue" : "Create your account"}
          </p>
        </div>

        <form className="space-y-4" noValidate onSubmit={submit}>
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
              {...fieldProps("username")}
              type="text"
              autoComplete="username"
              value={username}
              onChange={(e) => {
                setUsername(e.target.value);
                clearError("username");
              }}
              onBlur={() => invalidate("username")}
              autoFocus
              className={inputCls(!!errors.username)}
            />
            {errors.username && (
              <p id="login-username-error" role="alert" className={errorCls}>
                {errors.username}
              </p>
            )}
          </div>
          <div>
            <label className={label} htmlFor="login-password">
              Password
            </label>
            <input
              {...fieldProps("password")}
              type="password"
              autoComplete={mode === "login" ? "current-password" : "new-password"}
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                clearError("password");
                if (mode === "register") clearError("confirm");
              }}
              onBlur={() => invalidate("password")}
              className={inputCls(!!errors.password)}
            />
            {errors.password && (
              <p id="login-password-error" role="alert" className={errorCls}>
                {errors.password}
              </p>
            )}
          </div>
          {mode === "register" && (
            <div>
              <label className={label} htmlFor="login-confirm">
                Confirm password
              </label>
              <input
                {...fieldProps("confirm")}
                type="password"
                autoComplete="new-password"
                value={confirm}
                onChange={(e) => {
                  setConfirm(e.target.value);
                  clearError("confirm");
                }}
                onBlur={() => invalidate("confirm")}
                className={inputCls(!!errors.confirm)}
              />
              {errors.confirm && (
                <p id="login-confirm-error" role="alert" className={errorCls}>
                  {errors.confirm}
                </p>
              )}
            </div>
          )}
          <Button className="w-full" loading={busy} type="submit">
            {mode === "register" ? "Create account" : "Log in"}
          </Button>
          <p className="text-center text-xs text-slate-500">
            {mode === "login" ? (
              <>
                New here?{" "}
                <button
                  type="button"
                  onClick={() => switchMode("register")}
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
                  onClick={() => switchMode("login")}
                  className="font-medium text-indigo-600 hover:underline"
                >
                  Log in
                </button>
              </>
            )}
          </p>
        </form>
      </div>
    </div>
  );
}