import { describe, it, expect } from "vitest";
import { explorerUrl } from "../lib/escrow";

describe("explorerUrl", () => {
  it("builds an etherscan tx link for mainnet", () => {
    const tx = "0x" + "a".repeat(64);
    const { name, url } = explorerUrl(1, tx);
    expect(name).toContain("etherscan");
    expect(url).toContain("/tx/");
    expect(url).toContain(tx);
  });

  it("builds a sepolia address link", () => {
    const addr = "0x1234567890abcdef1234567890abcdef12345678";
    const { url } = explorerUrl(11155111, addr);
    expect(url).toContain("sepolia.etherscan.io/address/");
    expect(url).toContain(addr);
  });

  it("returns empty url for unknown chains in mainnet id range", () => {
    const { url } = explorerUrl(99999, "0xabc");
    expect(url).toBe("");
  });
});
