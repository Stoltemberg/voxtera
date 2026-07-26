import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";

afterEach(cleanup);

describe("App", () => {
  it("renders the approved cinematic sections without card or thumbnail composition", () => {
    render(<App />);

    expect(document.querySelector(".feature-card")).not.toBeInTheDocument();
    expect(document.querySelector(".step-illustration")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Explore" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Aventure-se" })).toBeInTheDocument();
  });

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
