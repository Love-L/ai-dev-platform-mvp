import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import AnimatedNumber from "../AnimatedNumber";

// Mock react-countup
vi.mock("react-countup", () => ({
  default: ({ end, prefix, suffix }: any) => (
    <span>
      {prefix}
      {end}
      {suffix}
    </span>
  ),
}));

describe("AnimatedNumber component", () => {
  it("renders the target number", async () => {
    render(<AnimatedNumber value={42} />);
    await waitFor(() => {
      expect(screen.getByText("42")).toBeInTheDocument();
    });
  });

  it("renders with suffix", async () => {
    render(<AnimatedNumber value={100} suffix="MB" />);
    await waitFor(() => {
      expect(screen.getByText("100MB")).toBeInTheDocument();
    });
  });

  it("renders with prefix", async () => {
    render(<AnimatedNumber value={50} prefix="$" />);
    await waitFor(() => {
      expect(screen.getByText("$50")).toBeInTheDocument();
    });
  });

  it("renders zero initially before animation starts", () => {
    render(<AnimatedNumber value={42} />);
    expect(screen.getByText("0")).toBeInTheDocument();
  });
});
