// Backend -> frontend event payloads.

import type { AppStateEnum } from "./app";

export interface StateChangedEvent {
  state: AppStateEnum;
}

export interface TranscriptionCompleteEvent {
  id: string;
  text: string;
  word_count: number;
  duration_ms: number;
  /** False when the text never reached the focused app (saved to History only). */
  injected: boolean;
  /** Process name of the injection target, when known. */
  app_context?: string | null;
}

export interface TranscriptionErrorEvent {
  message: string;
  code: string;
}

export interface AudioLevelEvent {
  level: number;
}

export interface FileTranscriptionProgressEvent {
  stage: "transcribing" | "formatting";
  done: number;
  /** 0 when the file does not report its duration. */
  total: number;
}

export interface WidgetHideRequestedEvent {
  id: number;
}

export interface WidgetRevealRequestedEvent {
  id: number;
}

export interface NoticeEvent {
  code: string;
  message: string;
}
