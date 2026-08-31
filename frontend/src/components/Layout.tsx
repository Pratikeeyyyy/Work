import { NavLink, Outlet } from "react-router-dom";
import { Icon } from "./Icon";
import WalletButton from "./WalletButton";

const NAV = [
  { to: "/", label: "Dashboard", icon: "dashboard", end: true },
  { to: "/leads", label: "Leads", icon: "leads", end: false },
  { to: "/clients", label: "Clients", icon: "clients", end: false },
  { to: "/contracts", label: "Contracts", icon: "contracts", end: false },
  { to: "/settings", label: "Settings", icon: "settings", end: false },
];

function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <div className="flex items-center gap-2.5">
      <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-indigo-600 text-white">
        <Icon name="contracts" className="h-5 w-5" />
      </div>
      {!compact && (
        <div className="leading-tight">
          <p className="text-sm font-bold text-white">LeadGen</p>
          <p className="text-[11px] text-slate-400">deals · pipeline · escrow</p>
        </div>
      )}
    </div>
  );
}

function NavItems({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="flex gap-1 overflow-x-auto lg:flex-col lg:overflow-visible" aria-label="Primary">
      {NAV.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.end}
          onClick={onNavigate}
          className={({ isActive }) =>
            `flex shrink-0 items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-400 ${
              isActive
                ? "bg-indigo-600 text-white"
                : "text-slate-300 hover:bg-slate-800 hover:text-white"
            }`
          }
        >
          <Icon name={item.icon} className="h-5 w-5 shrink-0" />
          <span className="whitespace-nowrap">{item.label}</span>
        </NavLink>
      ))}
    </nav>
  );
}

export default function Layout() {
  return (
    <div className="min-h-screen lg:flex">
      <aside className="hidden w-60 shrink-0 flex-col bg-slate-900 lg:flex">
        <div className="px-5 pt-5">
          <Brand />
        </div>
        <div className="flex-1 overflow-y-auto px-4 py-5">
          <NavItems />
        </div>
        <div className="border-t border-slate-800 px-4 py-4">
          <WalletButton variant="dark" />
        </div>
      </aside>

      <div className="min-w-0 flex-1">
        <header className="sticky top-0 z-30 border-b border-slate-200 bg-white/85 backdrop-blur lg:hidden">
          <div className="flex items-center gap-3 px-4 py-3">
            <Brand compact />
            <div className="ml-auto">
              <WalletButton variant="light" />
            </div>
          </div>
          <div className="px-4 pb-2.5">
            <NavItems />
          </div>
        </header>

        <main className="mx-auto w-full max-w-7xl px-4 py-6 lg:px-8">
          <Outlet />
        </main>
      </div>
    </div>
  );
}