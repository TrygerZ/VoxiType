// Minimal navigation bus: lets non-React modules (toast actions, event
// handlers) switch the shell view without threading callbacks everywhere.

import type { View } from "../components/common/FloatingDock";

type Handler = (view: View, settingsTab?: string) => void;

let handler: Handler | null = null;

export function setNavigator(next: Handler | null) {
  handler = next;
}

export function navigate(view: View, settingsTab?: string) {
  handler?.(view, settingsTab);
}

let onboardingHandler: (() => void) | null = null;

export function setOnboardingHandler(next: (() => void) | null) {
  onboardingHandler = next;
}

export function openOnboarding() {
  onboardingHandler?.();
}
