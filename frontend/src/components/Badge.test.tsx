import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Badge, statusTone, displayLabel } from "../components/Badge";

describe("Badge", () => {
  it("renders children text", () => {
    render(<Badge>Active</Badge>);
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("maps a status to a tone (funded → sky)", () => {
    expect(statusTone("funded")).toBe("sky");
  });

  it("maps completed to emerald", () => {
    expect(statusTone("completed")).toBe("emerald");
  });

  it("uppercases source labels like FIVERR", () => {
    expect(displayLabel("fiverr")).toBe("FIVERR");
    expect(displayLabel("upwork")).toBe("UPWORK");
  });

  it("capitalizes normal statuses", () => {
    expect(displayLabel("in_progress")).toBe("In_progress");
    expect(displayLabel("completed")).toBe("Completed");
  });

  it("falls back to slate tone for unknown statuses", () => {
    expect(statusTone("mystery")).toBe("slate");
  });
});
