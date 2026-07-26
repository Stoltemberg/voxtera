import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("exposes the primary executable download action", () => {
    render(<App />);

    expect(screen.getByRole("link", {
      name: "Baixar launcher para Windows (.exe)",
    })).toHaveAttribute("href", "/download");
  });
});
