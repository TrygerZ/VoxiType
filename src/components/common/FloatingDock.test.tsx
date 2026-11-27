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

  it("renders shared sliding active pill, marks active item with aria-current=page, and omits backdrop blur or active dot", () => {
    const { container } = render(
      <FloatingDock active="history" onChange={vi.fn()} />,
    );

    const dock = container.firstElementChild as HTMLElement;
    expect(dock.className).toContain("bg-vx-bg-dock");
    expect(dock.className).not.toContain("backdrop-blur");
    expect(dock.className).not.toContain("/70");

    const activePill = screen.getByTestId("dock-active-pill");
    expect(activePill.className).toContain("bg-vx-bg-elevated");
    expect(activePill.className).toContain("border-vx-border-default");
    expect(activePill.className).toContain("duration-[240ms]");

    const historyBtn = screen.getByRole("button", { name: "History" });
    expect(historyBtn).toHaveAttribute("aria-current", "page");
    expect(historyBtn).toHaveTextContent("History");

    const homeBtn = screen.getByRole("button", { name: "Home" });
    expect(homeBtn).not.toHaveAttribute("aria-current");
    expect(dock.querySelector(".h-1.w-1")).toBeNull();
  });

  it("calls onChange with selected view id and updates active tab state on rerender", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const { rerender } = render(<FloatingDock active="home" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Snippets" }));
    expect(onChange).toHaveBeenCalledWith("snippets");

    rerender(<FloatingDock active="snippets" onChange={onChange} />);
    expect(screen.getByRole("button", { name: "Snippets" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("button", { name: "Home" })).not.toHaveAttribute(
      "aria-current",
    );
  });
});
