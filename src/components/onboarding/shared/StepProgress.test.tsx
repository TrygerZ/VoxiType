import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StepProgress } from "./StepProgress";

describe("StepProgress", () => {
  it("shows the setup counter for the active step", () => {
    const translate = (key: string, vars?: Record<string, string | number>) =>
      key === "onboarding.steps.counter"
        ? `${vars?.current} of ${vars?.total}`
        : key;

    render(<StepProgress step="stt_setup" currentStepIdx={3} t={translate} />);

    expect(screen.getByText("3 of 6")).toBeInTheDocument();
  });
});
