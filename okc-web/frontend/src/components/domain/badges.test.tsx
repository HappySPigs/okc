import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { SeverityBadge } from "./badges";

// Guards the product's core severity language (design-system §6): Major/Critical
// read as blocking; Minor as waivable.
describe("SeverityBadge", () => {
  it("renders the severity label for each level", () => {
    const { rerender } = render(<SeverityBadge severity="critical" />);
    expect(screen.getByText("Critical")).toBeInTheDocument();

    rerender(<SeverityBadge severity="major" />);
    expect(screen.getByText("Major")).toBeInTheDocument();

    rerender(<SeverityBadge severity="minor" />);
    expect(screen.getByText("Minor")).toBeInTheDocument();
  });

  it("fails closed on an unknown severity (shows the raw value, not Minor)", () => {
    render(<SeverityBadge severity="weird" />);
    expect(screen.getByText("weird")).toBeInTheDocument();
    expect(screen.queryByText("Minor")).not.toBeInTheDocument();
  });
});
