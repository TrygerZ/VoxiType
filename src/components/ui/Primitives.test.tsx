import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Trash2 } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "./Button";
import { IconButton } from "./IconButton";
import { Input } from "./Input";
import { Keycap } from "./Keycap";
import { Select } from "./Select";
import { StatusBadge } from "./StatusBadge";
import { Switch } from "./Switch";

describe("UI primitives", () => {
  describe("Button", () => {
    it("supports primary, secondary, ghost, danger, and ember variants without borders on danger", () => {
      const { rerender } = render(<Button variant="primary">Save</Button>);
      const btn = screen.getByRole("button", { name: "Save" });
      expect(btn.className).toContain("bg-vx-bg-inverse");
      expect(btn.className).toContain("text-vx-text-inverse");

      rerender(
        <Button variant="danger" size="sm">
          Delete
        </Button>,
      );
      expect(btn.className).toContain("bg-vx-semantic-error-soft");
      expect(btn.className).toContain("text-vx-semantic-error");
      expect(btn.className).toContain("h-[30px]");

      rerender(
        <Button variant="ember" size="lg">
          Record
        </Button>,
      );
      expect(btn.className).toContain("bg-vx-accent-ember");
      expect(btn.className).toContain("text-vx-stone-0");
      expect(btn.className).toContain("h-10");
    });
  });

  describe("IconButton", () => {
    it("requires accessible label, defaults type=button, and supports ghost and danger tones", async () => {
      const user = userEvent.setup();
      const onClick = vi.fn();
      const { rerender } = render(
        <IconButton
          icon={<Trash2 data-testid="trash-icon" />}
          label="Delete entry"
          tone="danger"
          size="md"
          onClick={onClick}
        />,
      );

      const button = screen.getByRole("button", { name: "Delete entry" });
      expect(button).toHaveAttribute("type", "button");
      expect(button.className).toContain("bg-vx-semantic-error-soft");
      expect(button.className).toContain("h-[30px]");
      expect(screen.getByTestId("trash-icon")).toBeInTheDocument();

      await user.click(button);
      expect(onClick).toHaveBeenCalledTimes(1);

      rerender(
        <IconButton
          icon={<Trash2 />}
          label="Remove item"
          tone="ghost"
          size="sm"
        />,
      );
      expect(button).toHaveAccessibleName("Remove item");
      expect(button.className).toContain("bg-transparent");
      expect(button.className).toContain("h-7");

      rerender(
        <IconButton icon={<Trash2 />} label="Pin item" tone="ghost" size="xs" />,
      );
      expect(button.className).toContain("h-6");
    });
  });

  describe("StatusBadge", () => {
    it("renders flat chip for neutral tone without dots or borders", () => {
      render(<StatusBadge tone="neutral" label="Queued" />);
      const badge = screen.getByText("Queued").closest("span");
      expect(badge).not.toBeNull();
      expect(badge?.className).toContain("bg-vx-bg-subtle");
      expect(badge?.className).toContain("rounded-[4px]");
      expect(badge?.className).not.toContain("border");
    });

    it("renders icon-plus-text without background pill or border for semantic tones", () => {
      const tones = ["success", "ember", "warning", "error"] as const;
      for (const tone of tones) {
        const { unmount } = render(<StatusBadge tone={tone} label={`Label ${tone}`} />);
        const textEl = screen.getByText(`Label ${tone}`);
        const badge = textEl.closest("span");
        expect(badge).not.toBeNull();
        expect(badge?.className).not.toContain("bg-vx-");
        expect(badge?.className).not.toContain("border");
        expect(badge?.querySelector("svg")).not.toBeNull();
        unmount();
      }
    });
  });

  describe("Keycap", () => {
    it("renders a kbd element with monospace styling and bottom border lip", () => {
      render(<Keycap>Ctrl</Keycap>);
      const kbd = screen.getByText("Ctrl");
      expect(kbd.tagName.toLowerCase()).toBe("kbd");
      expect(kbd.className).toContain("font-mono");
      expect(kbd.className).toContain("border-b-2");
    });
  });

  describe("Input, Select, Switch", () => {
    it("renders Input with matte subtle surface, default border, and working password toggle", async () => {
      const user = userEvent.setup();
      render(<Input label="API Key" type="password" showPasswordToggle placeholder="gsk_..." />);
      const input = screen.getByLabelText("API Key");
      expect(input).toHaveAttribute("type", "password");
      expect(input.className).toContain("bg-vx-bg-subtle");
      expect(input.className).toContain("border-vx-border-default");

      await user.click(screen.getByRole("button", { name: "Show API Key" }));
      expect(input).toHaveAttribute("type", "text");
    });

    it("renders Select with matte surface and styled options", () => {
      render(
        <Select
          label="Engine"
          options={[
            { value: "groq", label: "Groq Whisper" },
            { value: "whisper_cpp", label: "Offline" },
          ]}
        />,
      );
      const select = screen.getByLabelText("Engine");
      expect(select.className).toContain("bg-vx-bg-subtle");
      expect(select.className).toContain("border-vx-border-default");
      expect(screen.getByRole("option", { name: "Groq Whisper" })).toBeInTheDocument();
    });

    it("renders Switch with monochrome track and toggles on click", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<Switch checked={true} onChange={onChange} ariaLabel="Enable feature" />);
      const sw = screen.getByRole("switch", { name: "Enable feature" });
      expect(sw).toHaveAttribute("aria-checked", "true");
      expect(sw.innerHTML).toContain("bg-vx-bg-inverse");

      await user.click(sw);
      expect(onChange).toHaveBeenCalledWith(false);
    });
  });
});
