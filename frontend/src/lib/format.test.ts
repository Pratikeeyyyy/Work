import { describe, it, expect } from "vitest";
import { formatWei, shortAddress, joinTags, timeAgo } from "../lib/format";

describe("formatWei", () => {
  it("formats whole ETH", () => {
    expect(formatWei("1000000000000000000")).toBe("1 ETH");
  });

  it("formats fractional ETH and trims trailing zeros", () => {
    expect(formatWei("1500000000000000000")).toBe("1.5 ETH");
  });

  it("handles null/undefined as dash", () => {
    expect(formatWei(null)).toBe("—");
    expect(formatWei(undefined)).toBe("—");
  });

  it("returns raw string on invalid input", () => {
    expect(formatWei("not-a-number")).toBe("not-a-number");
  });
});

describe("shortAddress", () => {
  it("abbreviates a long address", () => {
    expect(shortAddress("0x1234567890abcdef")).toBe("0x1234…cdef");
  });

  it("handles null as dash", () => {
    expect(shortAddress(null)).toBe("—");
  });
});

describe("joinTags", () => {
  it("splits and trims comma-separated tags", () => {
    expect(joinTags(" rust, solidity ,  python ")).toEqual(["rust", "solidity", "python"]);
  });

  it("returns empty array for null", () => {
    expect(joinTags(null)).toEqual([]);
  });
});

describe("timeAgo", () => {
  it("returns just now for recent timestamps", () => {
    expect(timeAgo(new Date().toISOString())).toBe("just now");
  });

  it("returns dash for null", () => {
    expect(timeAgo(null)).toBe("—");
  });
});
