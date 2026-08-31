import { useCallback, useEffect, useMemo, useState } from "react";
import { ethers } from "ethers";
import { api } from "../api";
import type { Client, ContractRow, NewContract } from "../types";
import { formatDate, formatWei, shortAddress } from "../lib/format";
import { deployFreelanceEscrow, explorerUrl } from "../lib/escrow";
import { useWallet } from "../lib/wallet";
import { Badge, statusTone } from "../components/Badge";
import Button from "../components/Button";
import Spinner from "../components/Spinner";
import EmptyState from "../components/EmptyState";
import Modal from "../components/Modal";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

export default function Contracts() {
  const [contracts, setContracts] = useState<ContractRow[]>([]);
  const [clients, setClients] = useState<Client[]>([]);
  const [loading, setLoading] = useState(true);
  const [createOpen, setCreateOpen] = useState(false);
  const [deployTarget, setDeployTarget] = useState<ContractRow | null>(null);
  const { account, chainId, provider } = useWallet();
  const { notify } = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [c, cl] = await Promise.all([api.listContracts(), api.listClients()]);
      setContracts(c);
      setClients(cl);
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to load contracts", "error");
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  const clientById = useMemo(
    () => new Map(clients.map((c) => [c.id, c])),
    [clients],
  );

  const deploy = useCallback(
    async (target: ContractRow) => {
      if (!target.freelancer_address || !ethers.isAddress(target.freelancer_address)) {
        notify("Set a valid freelancer address on the contract first", "error");
        return;
      }
      if (!target.amount_wei || BigInt(target.amount_wei) <= 0n) {
        notify("Contract has no funded amount. Enter the escrow amount ETH.", "error");
        return;
      }
      if (target.currency !== "ETH") {
        notify("Only ETH escrows are supported for on-chain deploy", "error");
        return;
      }
      if (!provider || !account) {
        notify("Connect your wallet to deploy the escrow", "error");
        return;
      }
      const signer = await provider.getSigner();
      notify("Waiting for wallet confirmation…", "info");
      const { contractAddress, txHash } = await deployFreelanceEscrow(
        signer,
        target.freelancer_address,
        target.notes ?? "",
        BigInt(target.amount_wei),
      );
      await api.deployContract(target.id, { tx_hash: txHash, contract_address: contractAddress });
      notify(`Escrow deployed at ${shortAddress(contractAddress)}`);
      setDeployTarget(null);
      void load();
    },
    [account, provider, notify, load],
  );

  if (loading) {
    return (
      <div className="grid place-items-center py-24">
        <Spinner className="h-8 w-8" />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-bold text-slate-900 sm:text-2xl">Contracts</h1>
          <p className="text-sm text-slate-500">Agreed engagements, protected by on-chain escrow.</p>
        </div>
        <Button size="sm" icon={<Icon name="plus" className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          New contract
        </Button>
      </div>

      {contracts.length === 0 ? (
        <EmptyState
          icon="contracts"
          title="No contracts yet"
          hint="Create a contract against an existing client, then deploy the escrow from your wallet with the deposit."
        />
      ) : (
        <div className="space-y-3">
          {contracts.map((contract) => {
            const client = clientById.get(contract.client_id);
            const isClientDeployer =
              !!account && account.toLowerCase() === (contract.client_address ?? "").toLowerCase();
            return (
              <article
                key={contract.id}
                className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h2 className="font-semibold text-slate-900">{contract.title}</h2>
                      <Badge tone={statusTone(contract.status)}>{contract.status}</Badge>
                    </div>
                    <p className="mt-0.5 text-sm text-slate-500">
                      {client ? client.name : `Client #${contract.client_id}`} · created{" "}
                      {formatDate(contract.created_at)}
                    </p>
                  </div>
                  <p className="rounded-lg bg-slate-100 px-3 py-1.5 text-sm font-semibold text-slate-800">
                    {formatWei(contract.amount_wei)}
                  </p>
                </div>

                <div className="mt-4 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
                  <AddressField label="Client address" value={contract.client_address} />
                  <AddressField label="Freelancer address" value={contract.freelancer_address} />
                  <AddressField label="Escrow contract" value={contract.contract_address} chainId={chainId} />
                </div>

                <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-slate-100 pt-4">
                  <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-slate-500">
                    {contract.tx_hash ? (
                      <ChainLink
                        chainId={chainId}
                        hashOrAddress={contract.tx_hash}
                        label={shortAddress(contract.tx_hash)}
                      />
                    ) : (
                      <span>Transaction: —</span>
                    )}
                    {contract.deployed_at ? (
                      <span>Deployed {formatDate(contract.deployed_at)}</span>
                    ) : null}
                  </div>
                  {contract.status === "draft" && (
                    <Button
                      size="sm"
                      icon={<Icon name="contracts" className="h-4 w-4" />}
                      onClick={() => {
                        if (!account) {
                          notify("Connect your wallet to deploy the escrow", "error");
                          return;
                        }
                        setDeployTarget(contract);
                      }}
                      disabled={!account}
                      title={account ? "Deploy escrow" : "Connect wallet to deploy"}
                    >
                      Deploy escrow
                    </Button>
                  )}
                </div>

                {isClientDeployer && contract.status === "draft" && (
                  <p className="mt-3 rounded-lg bg-sky-50 px-3 py-2 text-xs text-sky-700">
                    You are the recorded client — deploying will lock your deposit in escrow.
                  </p>
                )}
              </article>
            );
          })}
        </div>
      )}

      <CreateContractModal
        open={createOpen}
        clients={clients}
        defaultClientAddress={account ?? ""}
        onClose={() => setCreateOpen(false)}
        onSaved={() => {
          setCreateOpen(false);
          void load();
        }}
      />

      <DeployModal
        target={deployTarget}
        chainId={chainId}
        onClose={() => setDeployTarget(null)}
        onDeploy={async (target) => {
          try {
            await deploy(target);
          } catch (e) {
            notify(e instanceof Error ? e.message : "Deployment failed", "error");
          }
        }}
      />
    </div>
  );
}

function AddressField({
  label,
  value,
  chainId,
}: {
  label: string;
  value: string | null;
  chainId?: number | null;
}) {
  const { notify } = useToast();
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-slate-500">{label}</span>
      {value ? (
        <span className="flex items-center gap-2">
          <ChainLink chainId={chainId} hashOrAddress={value} label={shortAddress(value)} />
          <button
            onClick={() => {
              void navigator.clipboard.writeText(value);
              notify("Address copied");
            }}
            className="text-slate-400 transition-colors hover:text-indigo-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 rounded"
            aria-label="Copy address"
          >
            <Icon name="link" className="h-3.5 w-3.5" />
          </button>
        </span>
      ) : (
        <span className="text-slate-400">—</span>
      )}
    </div>
  );
}

function ChainLink({
  chainId,
  hashOrAddress,
  label,
}: {
  chainId?: number | null;
  hashOrAddress: string;
  label: string;
}) {
  const { url } = explorerUrl(chainId ?? null, hashOrAddress);
  if (!url) {
    return <span className="font-mono text-xs text-slate-600">{label}</span>;
  }
  return (
    <a
      href={url}
      target="_blank"
      rel="noreferrer noopener"
      className="inline-flex items-center gap-1 font-mono text-xs text-indigo-600 hover:text-indigo-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 rounded"
    >
      {label}
      <Icon name="external" className="h-3 w-3" />
    </a>
  );
}

function CreateContractModal({
  open,
  clients,
  defaultClientAddress,
  onClose,
  onSaved,
}: {
  open: boolean;
  clients: Client[];
  defaultClientAddress: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<NewContract>({
    client_id: 0,
    client_address: null,
    freelancer_address: null,
    contract_address: null,
    title: "",
    amount_wei: null,
    currency: "ETH",
    notes: null,
  });
  const [amountEth, setAmountEth] = useState("");
  const [saving, setSaving] = useState(false);
  const { notify } = useToast();

  useEffect(() => {
    if (open) {
      setForm({
        client_id: clients[0]?.id ?? 0,
        client_address: defaultClientAddress ? defaultClientAddress.toLowerCase() : null,
        freelancer_address: null,
        contract_address: null,
        title: "",
        amount_wei: null,
        currency: "ETH",
        notes: null,
      });
      setAmountEth("");
    }
  }, [open, clients, defaultClientAddress]);

  const submit = async () => {
    if (!form.client_id) {
      notify("Select a client", "error");
      return;
    }
    if (!form.title.trim()) {
      notify("Title is required", "error");
      return;
    }
    if (!form.freelancer_address || !ethers.isAddress(form.freelancer_address)) {
      notify("Enter a valid freelancer address (you)", "error");
      return;
    }
    let wei: string;
    try {
      wei = ethers.parseEther(amountEth || "0").toString();
    } catch {
      notify("Enter a valid escrow amount in ETH", "error");
      return;
    }
    if (BigInt(wei) <= 0n) {
      notify("Escrow amount must be greater than 0 ETH", "error");
      return;
    }
    setSaving(true);
    try {
      await api.addContract({
        ...form,
        title: form.title.trim(),
        client_address: form.client_address?.toLowerCase() ?? null,
        freelancer_address: form.freelancer_address.toLowerCase(),
        amount_wei: wei,
      });
      notify("Contract created — deploy the escrow when both sides agree");
      onSaved();
    } catch (e) {
      notify(e instanceof Error ? e.message : "Failed to create contract", "error");
    } finally {
      setSaving(false);
    }
  };

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <Modal
      open={open}
      title="New escrow contract"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={saving} onClick={submit}>
            Create contract
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label className={label} htmlFor="contract-client">
            Client
          </label>
          <select
            id="contract-client"
            className={input}
            value={form.client_id}
            onChange={(e) => setForm((f) => ({ ...f, client_id: Number(e.target.value) }))}
          >
            <option value={0}>Select a client…</option>
            {clients.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          {clients.length === 0 && (
            <p className="mt-2 text-xs text-slate-500">
              No clients yet — convert a lead to a client first.
            </p>
          )}
        </div>

        <div>
          <label className={label} htmlFor="contract-title">
            Title
          </label>
          <input
            id="contract-title"
            className={input}
            value={form.title}
            onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
            placeholder="Milestone 1 — Auth dashboard"
          />
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="contract-client-addr">
              Client address
            </label>
            <input
              id="contract-client-addr"
              className={input}
              value={form.client_address ?? ""}
              onChange={(e) => setForm((f) => ({ ...f, client_address: e.target.value || null }))}
              placeholder="0x… (funding wallet)"
            />
          </div>
          <div>
            <label className={label} htmlFor="contract-freelancer-addr">
              Freelancer address *
            </label>
            <input
              id="contract-freelancer-addr"
              className={input}
              value={form.freelancer_address ?? ""}
              onChange={(e) => setForm((f) => ({ ...f, freelancer_address: e.target.value || null }))}
              placeholder="0x… (your wallet)"
            />
          </div>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className={label} htmlFor="contract-amount">
              Amount (ETH) *
            </label>
            <input
              id="contract-amount"
              type="number"
              min="0"
              step="0.01"
              className={input}
              value={amountEth}
              onChange={(e) => setAmountEth(e.target.value)}
              placeholder="0.5"
            />
          </div>
          <div>
            <label className={label} htmlFor="contract-currency">
              Currency
            </label>
            <select
              id="contract-currency"
              className={input}
              value={form.currency}
              onChange={(e) => setForm((f) => ({ ...f, currency: e.target.value }))}
            >
              <option value="ETH">ETH</option>
              <option value="USD">USD (off-chain)</option>
              <option value="EUR">EUR (off-chain)</option>
            </select>
          </div>
        </div>

        <div>
          <label className={label} htmlFor="contract-notes">
            Mediator address / notes
          </label>
          <input
            id="contract-notes"
            className={input}
            value={form.notes ?? ""}
            onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value || null }))}
            placeholder="Optional dispute mediator address (0x…) or any notes"
          />
          <p className="mt-2 text-xs text-slate-500">
            You will be asked for a mediator address when deploying the escrow.
          </p>
        </div>
      </div>
    </Modal>
  );
}

function DeployModal({
  target,
  chainId,
  onClose,
  onDeploy,
}: {
  target: ContractRow | null;
  chainId: number | null;
  onClose: () => void;
  onDeploy: (target: ContractRow) => void;
}) {
  const [mediator, setMediator] = useState("");
  const [deploying, setDeploying] = useState(false);
  const { account } = useWallet();
  const { notify } = useToast();

  useEffect(() => {
    if (target) setMediator("");
  }, [target]);

  if (!target) return null;

  const deploy = () => {
    if (mediator && !ethers.isAddress(mediator)) {
      notify("Mediator address is invalid (or leave empty)", "error");
      return;
    }
    setDeploying(true);
    try {
      onDeploy(target);
    } finally {
      setDeploying(false);
    }
  };

  const deployerMismatch =
    !!account &&
    !!target.client_address &&
    account.toLowerCase() !== target.client_address.toLowerCase();

  const input = "mt-1 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/20";
  const label = "block text-sm font-medium text-slate-700";

  return (
    <Modal
      open
      title="Deploy escrow"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={deploying}>
            Cancel
          </Button>
          <Button loading={deploying} icon={<Icon name="wallet" className="h-4 w-4" />} onClick={deploy}>
            Deploy & deposit
          </Button>
        </>
      }
    >
      <dl className="space-y-2 rounded-lg bg-slate-50 p-4 text-sm">
        <div className="flex justify-between">
          <dt className="text-slate-500">Project</dt>
          <dd className="font-medium text-slate-900">{target.title}</dd>
        </div>
        <div className="flex justify-between">
          <dt className="text-slate-500">Deposit</dt>
          <dd className="font-medium text-slate-900">{formatWei(target.amount_wei)}</dd>
        </div>
        <div className="flex justify-between">
          <dt className="text-slate-500">Deployer (client)</dt>
          <dd className="font-mono text-xs text-slate-700">{account ? shortAddress(account) : "—"}</dd>
        </div>
        <div className="flex justify-between">
          <dt className="text-slate-500">Freelancer</dt>
          <dd className="font-mono text-xs text-slate-700">{shortAddress(target.freelancer_address)}</dd>
        </div>
      </dl>

      {deployerMismatch && (
        <p className="mt-3 rounded-lg bg-amber-50 px-3 py-2 text-xs text-amber-800">
          The connected wallet is not the client address stored on this contract. The escrow's
          client will be your connected account.
        </p>
      )}

      <div className="mt-4">
        <label className={label} htmlFor="deploy-mediator">
          Mediator address (optional)
        </label>
        <input
          id="deploy-mediator"
          className={input}
          value={mediator}
          onChange={(e) => setMediator(e.target.value)}
          placeholder="0x… dispute resolver, or leave empty"
        />
        <p className="mt-2 text-xs text-slate-500">
          Mediator resolves disputes by splitting funds. Without one, disputes stay locked until the
          client approves or the deadline refunds. Explorer:{" "}
          {chainId ? `chain ${chainId}` : "connect wallet to see links"}.
        </p>
      </div>
    </Modal>
  );
}