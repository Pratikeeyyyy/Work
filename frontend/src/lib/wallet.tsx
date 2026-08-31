import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ethers } from "ethers";

interface Eip1193 {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
  on?: (event: string, callback: (...args: unknown[]) => void) => void;
  removeListener?: (event: string, callback: (...args: unknown[]) => void) => void;
}

interface WalletCtx {
  account: string | null;
  chainId: number | null;
  provider: ethers.BrowserProvider | null;
  connecting: boolean;
  connect: () => Promise<void>;
  disconnect: () => void;
}

const WalletContext = createContext<WalletCtx | null>(null);

export function WalletProvider({ children }: { children: ReactNode }) {
  const [account, setAccount] = useState<string | null>(null);
  const [chainId, setChainId] = useState<number | null>(null);
  const [provider, setProvider] = useState<ethers.BrowserProvider | null>(null);
  const [connecting, setConnecting] = useState(false);

  const eth = (window as unknown as { ethereum?: Eip1193 }).ethereum;

  useEffect(() => {
    if (!eth) return;
    const onAccounts = (...args: unknown[]) => {
      const accounts = args[0] as string[];
      setAccount(accounts[0] ?? null);
      if (!accounts.length) setProvider(null);
    };
    const onChain = (...args: unknown[]) => setChainId(Number(args[0]));
    eth.on?.("accountsChanged", onAccounts);
    eth.on?.("chainChanged", onChain);
    return () => {
      eth.removeListener?.("accountsChanged", onAccounts);
      eth.removeListener?.("chainChanged", onChain);
    };
  }, [eth]);

  const connect = async () => {
    if (!eth) throw new Error("No wallet detected. Install MetaMask to deploy escrows.");
    setConnecting(true);
    try {
      const accounts = (await eth.request({ method: "eth_requestAccounts" })) as string[];
      const nextProvider = new ethers.BrowserProvider(eth as ethers.Eip1193Provider);
      const network = await nextProvider.getNetwork();
      setAccount(accounts[0] ?? null);
      setChainId(Number(network.chainId));
      setProvider(nextProvider);
    } finally {
      setConnecting(false);
    }
  };

  const disconnect = () => {
    setAccount(null);
    setChainId(null);
    setProvider(null);
  };

  const value = useMemo(
    () => ({ account, chainId, provider, connecting, connect, disconnect }),
    [account, chainId, provider, connecting],
  );

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet(): WalletCtx {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error("useWallet must be used within a <WalletProvider>");
  return ctx;
}