import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("exposes the primary executable download action", () => {
    render(<App />);

    const downloadActions = screen.getAllByRole("link", {
      name: "Baixar launcher para Windows (.exe)",
    });

    expect(downloadActions).toHaveLength(2);
    downloadActions.forEach((action) => {
      expect(action).toHaveAttribute("href", "/downloads/VoxteraLauncher.exe");
    });
  });

  it("does not expose GitHub links", () => {
    render(<App />);
    expect(screen.queryByRole("link", { name: /github/i })).not.toBeInTheDocument();
  });
});
