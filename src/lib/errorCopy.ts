// Maps backend ErrorCode values to a user-facing cause plus a recovery route.
// Unknown codes fall back to the backend message unchanged.

import { t } from "./i18n";

export interface ErrorCopy {
  message: string;
  /** Settings tab to open for recovery, when the cause has one. */
  settingsTab?: string;
}

const CODE_MAP: Record<string, { key: string; settingsTab?: string }> = {
  SttApiKeyInvalid: { key: "error.stt_api_key_invalid", settingsTab: "stt" },
  LlmApiKeyInvalid: { key: "error.llm_api_key_invalid", settingsTab: "llm" },
  MicNotFound: { key: "error.mic_unavailable", settingsTab: "audio" },
  MicPermissionDenied: { key: "error.mic_unavailable", settingsTab: "audio" },
  AudioDeviceError: { key: "error.mic_unavailable", settingsTab: "audio" },
  InjectionFailed: { key: "error.injection_failed" },
  InjectionPermission: { key: "error.injection_failed" },
};

export function errorCopy(code: string, fallback: string): ErrorCopy {
  const mapped = CODE_MAP[code];
  if (!mapped) return { message: fallback };
  return { message: t(mapped.key), settingsTab: mapped.settingsTab };
}
