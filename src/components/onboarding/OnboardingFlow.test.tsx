import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { OnboardingFlow } from "./OnboardingFlow";

describe("OnboardingFlow", () => {
  it("renders the welcome step", () => {
    render(<OnboardingFlow onComplete={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Welcome to VoxiType" })).toBeInTheDocument();
  });

  it("opens quick settings from welcome", async () => {
    const user = userEvent.setup();
    render(<OnboardingFlow onComplete={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Start Setup" }));

    expect(screen.getByRole("heading", { name: "Quick Settings" })).toBeInTheDocument();
  });

  it("calls onComplete when welcome is skipped", async () => {
    const user = userEvent.setup();
    const onComplete = vi.fn();
    render(<OnboardingFlow onComplete={onComplete} />);

    await user.click(screen.getByRole("button", { name: "Skip" }));

    expect(onComplete).toHaveBeenCalledOnce();
  });
});
