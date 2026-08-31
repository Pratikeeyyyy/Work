import { ethers } from "ethers";
import artifactJson from "./FreelanceEscrow.json";

const artifact = artifactJson as { abi: unknown[]; bytecode: string };

export interface DeployResult {
  contractAddress: string;
  txHash: string;
}

// Contract-literal escrow states: Funded(0) InProgress(1) Submitted(2)
// Completed(3) Disputed(4) Refunded(5)
export interface EscrowInfo {
  client: string;
  freelancer: string;
  mediator: string;
  amount: bigint;
  deadline: bigint;
  state: number;
  submittedAt: bigint;
}

export type EscrowState = "funded" | "in_progress" | "submitted" | "completed" | "disputed" | "refunded";

export const ESCROW_STATES: EscrowState[] = [
  "funded",
  "in_progress",
  "submitted",
  "completed",
  "disputed",
  "refunded",
];

export function contractFor(
  signerOrProvider: ethers.Signer | ethers.Provider,
  address: string,
): ethers.Contract {
  return new ethers.Contract(address, artifact.abi as ethers.InterfaceAbi, signerOrProvider);
}

async function sendWait(tx: Promise<unknown>): Promise<string | null> {
  const t = (await tx) as { wait?: () => Promise<{ hash?: string } | null> };
  const receipt = await t.wait?.();
  return receipt?.hash ?? null;
}

/**
 * Deploys contracts/FreelanceEscrow.sol as the client (sender), depositing
 * amountWei in the same transaction. Returns the deployed address + tx hash.
 */
export async function deployFreelanceEscrow(
  signer: ethers.Signer,
  freelancer: string,
  mediator: string,
  amountWei: bigint,
): Promise<DeployResult> {
  if (amountWei <= 0n) throw new Error("Escrow amount must be greater than 0");
  if (!ethers.isAddress(freelancer)) throw new Error("Invalid freelancer address");
  const mediatorAddr = mediator && ethers.isAddress(mediator) ? mediator : ethers.ZeroAddress;

  const factory = new ethers.ContractFactory(
    artifact.abi as ethers.InterfaceAbi,
    artifact.bytecode,
    signer,
  );
  const tx = await factory.getDeployTransaction(freelancer, mediatorAddr, {
    value: amountWei,
  });
  const sent = await signer.sendTransaction(tx);
  const receipt = await sent.wait();
  if (!receipt?.contractAddress) throw new Error("Deployment failed: no receipt returned");
  return { contractAddress: receipt.contractAddress, txHash: receipt.hash };
}

export function explorerUrl(
  chainId: number | null,
  hashOrAddress: string,
): { name: string; url: string } {
  const prefix: Record<number, string> = {
    1: "https://etherscan.io",
    11155111: "https://sepolia.etherscan.io",
    137: "https://polygonscan.com",
    8453: "https://basescan.org",
    42161: "https://arbiscan.io",
  };
  const base = chainId ? prefix[chainId] : undefined;
  if (!base) return { name: `chain ${chainId ?? "?"}`, url: "" };
  const kind = hashOrAddress.length === 66 ? "tx" : "address";
  return { name: base.replace("https://", ""), url: `${base}/${kind}/${hashOrAddress}` };
}

export async function getEscrowInfo(
  provider: ethers.Provider,
  contractAddress: string,
): Promise<EscrowInfo> {
  const c = contractFor(provider, contractAddress);
  const [client, freelancer, mediator, amount, deadline, state, submittedAt] =
    await c.getInfo();
  return {
    client: client as string,
    freelancer: freelancer as string,
    mediator: mediator as string,
    amount: amount as bigint,
    deadline: deadline as bigint,
    state: state as number,
    submittedAt: submittedAt as bigint,
  };
}

export async function startWork(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.startWork())) ?? "";
}

export async function submitWork(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.submitWork())) ?? "";
}

export async function approveWork(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.approve())) ?? "";
}

export async function cancelBeforeWork(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.cancelBeforeWork())) ?? "";
}

export async function raiseDispute(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.raiseDispute())) ?? "";
}

export async function refundAfterDeadline(
  signer: ethers.Signer,
  contractAddress: string,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.refundAfterDeadline())) ?? "";
}

export async function resolveDispute(
  signer: ethers.Signer,
  contractAddress: string,
  freelancerShareWei: bigint,
): Promise<string> {
  const c = contractFor(signer, contractAddress);
  return (await sendWait(c.resolveDispute(freelancerShareWei))) ?? "";
}