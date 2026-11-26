import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StepProgress } from "./StepProgress";

describe("StepProgress", () => {
  it("shows the 8-step counter and 8 segments for the active step", () => {
    const translate = (key: string, vars?: Record<string, string | number>) =>
      key === "onboarding.steps.counter"
        ? `Step ${vars?.current} of ${vars?.total}`
        : key;

    render(<StepProgress step="stt_setup" currentStepIdx={3} t={translate} />);

    expect(screen.getByText("Step 4 of 8")).toBeInTheDocument();
    const progressbar = screen.getByRole("progressbar");
    expect(progressbar).toHaveAttribute("aria-valuemax", "8");
    expect(progressbar).toHaveAttribute("aria-valuenow", "4");
    expect(progressbar.children).toHaveLength(8);
  });
});
