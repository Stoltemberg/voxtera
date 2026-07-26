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

    const downloadActions = [
      screen.getByRole("link", { name: "Baixar" }),
      ...screen.getAllByRole("link", {
        name: "Baixar launcher para Windows (.exe)",
      }),
      screen.getByRole("link", { name: "Baixar launcher" }),
    ];

    expect(downloadActions).toHaveLength(4);
    downloadActions.forEach((action) => {
      expect(action).toHaveAttribute("href", "/downloads/VoxteraLauncher.exe");
    });
  });

  it("renders the approved hero artwork with a direct launcher download", () => {
    render(<App />);

    expect(screen.getByAltText("Vale ensolarado de Voxtera com aventureiro e vila")).toHaveAttribute(
      "src",
      "/images/voxtera-clean-hero.png",
    );
    expect(screen.getAllByRole("link", { name: "Baixar launcher para Windows (.exe)" })[0]).toHaveAttribute(
      "href",
      "/downloads/VoxteraLauncher.exe",
    );
  });

  it("does not expose GitHub links", () => {
    render(<App />);

    document.querySelectorAll<HTMLAnchorElement>("a").forEach((link) => {
      expect(link.href).not.toContain("github.com");
    });
  });
});
