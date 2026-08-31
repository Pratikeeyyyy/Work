import { useCallback, useEffect, useState } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Leads from "./pages/Leads";
import Applications from "./pages/Applications";
import Discover from "./pages/Discover";
import Clients from "./pages/Clients";
import Contracts from "./pages/Contracts";
import Settings from "./pages/Settings";
import LinkedinCallback from "./pages/LinkedinCallback";
import Login from "./pages/Login";
import { api, auth } from "./api";
import Spinner from "./components/Spinner";
import { useToast } from "./components/Toast";

type AuthState = "checking" | "authed" | "guest";

export default function App() {
  const [authState, setAuthState] = useState<AuthState>("checking");
  const { notify } = useToast();

  const check = useCallback(() => {
    setAuthState("checking");
    void api
      .authStatus()
      .then((s) => setAuthState(s.authenticated && Boolean(auth.getToken()) ? "authed" : "guest"))
      .catch(() => {
        // Backend unreachable: let the app try to render; pages will surface errors.
        setAuthState(auth.getToken() ? "authed" : "guest");
      });
  }, []);

  useEffect(() => {
    check();
    const onUnauthorized = () => {
      auth.clearToken();
      setAuthState("guest");
      notify("Session expired — please log in again.", "error");
    };
    window.addEventListener("leadgen:unauthorized", onUnauthorized);
    return () => window.removeEventListener("leadgen:unauthorized", onUnauthorized);
  }, [check, notify]);

  if (authState === "checking") {
    return (
      <div className="grid min-h-screen place-items-center bg-slate-50">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  if (authState === "guest") {
    return <Login />;
  }

  return (
    <Routes>
      <Route path="/linkedin/callback" element={<LinkedinCallback />} />
      <Route element={<Layout />}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/leads" element={<Leads />} />
        <Route path="/applications" element={<Applications />} />
        <Route path="/discover" element={<Discover />} />
        <Route path="/clients" element={<Clients />} />
        <Route path="/contracts" element={<Contracts />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
