import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { FloatingDock } from "./FloatingDock";

describe("FloatingDock", () => {
  it("renders 6 items in grouped order separated by 2 hairline dividers", () => {
    const { container } = render(
      <FloatingDock active="home" onChange={vi.fn()} />,
    );

    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.getAttribute("aria-label"))).toEqual([
      "Home",
      "File",
      "History",
      "Dictionary",
      "Snippets",
      "Settings",
    ]);

    const dividers = container.querySelectorAll('[data-testid="dock-divider"]');
    expect(dividers).toHaveLength(2);
  });

  it("marks active item with aria-current=page, matte pill, and visible label without active dot or backdrop blur", () => {
    const { container } = render(
      <FloatingDock active="history" onChange={vi.fn()} />,
    );

    const dock = container.firstElementChild as HTMLElement;
    expect(dock.className).toContain("bg-vx-bg-dock");
    expect(dock.className).not.toContain("backdrop-blur");
    expect(dock.className).not.toContain("/70");

    const historyBtn = screen.getByRole("button", { name: "History" });
    expect(historyBtn).toHaveAttribute("aria-current", "page");
    expect(historyBtn.className).toContain("bg-vx-bg-elevated");
    expect(historyBtn).toHaveTextContent("History");

    const homeBtn = screen.getByRole("button", { name: "Home" });
    expect(homeBtn).not.toHaveAttribute("aria-current");
    expect(dock.querySelector(".h-1.w-1")).toBeNull();
  });

  it("calls onChange with selected view id", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<FloatingDock active="home" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Snippets" }));
    expect(onChange).toHaveBeenCalledWith("snippets");
  });
});
