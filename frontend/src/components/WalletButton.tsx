import { useWallet } from "../lib/wallet";
import { shortAddress } from "../lib/format";
import { useToast } from "./Toast";
import { Icon } from "./Icon";
import Button from "./Button";

export default function WalletButton({
  variant = "light",
}: {
  variant?: "light" | "dark";
}) {
  const { account, connecting, connect, disconnect } = useWallet();
  const { notify } = useToast();

  if (account) {
    return (
      <div className="flex items-center gap-1">
        <span
          title={account}
          className={`inline-flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs font-medium ${
            variant === "dark"
              ? "border-slate-700 bg-slate-800 text-slate-200"
              : "border-slate-200 bg-white text-slate-700"
          }`}
        >
          <span className="h-2 w-2 rounded-full bg-emerald-500" aria-hidden="true" />
          {shortAddress(account)}
        </span>
        <button
          onClick={disconnect}
          className="rounded-lg p-2 text-slate-400 transition-colors hover:bg-slate-100 hover:text-slate-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500"
          aria-label="Disconnect wallet"
        >
          <Icon name="close" className="h-4 w-4" />
        </button>
      </div>
    );
  }

  return (
    <Button
      variant={variant === "dark" ? "secondary" : "primary"}
      size="sm"
      loading={connecting}
      icon={<Icon name="wallet" className="h-4 w-4" />}
      onClick={async () => {
        try {
          await connect();
          notify("Wallet connected");
        } catch (e) {
          notify(e instanceof Error ? e.message : "Failed to connect wallet", "error");
        }
      }}
    >
      {connecting ? "Connecting" : "Connect wallet"}
    </Button>
  );
}