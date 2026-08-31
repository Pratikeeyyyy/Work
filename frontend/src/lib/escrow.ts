import { ethers } from "ethers";
import artifactJson from "./FreelanceEscrow.json";

const artifact = artifactJson as { abi: unknown[]; bytecode: string };

export interface DeployResult {
  contractAddress: string;
  txHash: string;
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