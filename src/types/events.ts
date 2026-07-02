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
}

export interface TranscriptionErrorEvent {
  message: string;
  code: string;
}

export interface AudioLevelEvent {
  level: number;
}
