import { Navigate, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Leads from "./pages/Leads";
import Applications from "./pages/Applications";
import Clients from "./pages/Clients";
import Contracts from "./pages/Contracts";
import Settings from "./pages/Settings";
import LinkedinCallback from "./pages/LinkedinCallback";

export default function App() {
  return (
    <Routes>
      <Route path="/linkedin/callback" element={<LinkedinCallback />} />
      <Route element={<Layout />}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/leads" element={<Leads />} />
        <Route path="/applications" element={<Applications />} />
        <Route path="/clients" element={<Clients />} />
        <Route path="/contracts" element={<Contracts />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}