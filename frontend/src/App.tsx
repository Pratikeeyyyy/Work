import { Navigate, Route, Routes } from "react-router-dom";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Leads from "./pages/Leads";
import Clients from "./pages/Clients";
import Contracts from "./pages/Contracts";
import Settings from "./pages/Settings";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/leads" element={<Leads />} />
        <Route path="/clients" element={<Clients />} />
        <Route path="/contracts" element={<Contracts />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
// Just say "continue the work" or "resume" — I'll check the current state of backend, contracts, and frontend, and pick up where we left off.
// To be more targeted, mention what you want next, e.g.:
// - "Continue — start the services and test the escrow flow end to end"
// - "Continue — finish the Fiverr scraper"
// - "Continue — deploy the contract to Sepolia"
// Want me to note specific next steps somewhere so resuming is unambiguous?