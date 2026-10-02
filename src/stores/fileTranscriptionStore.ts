import { create } from "zustand";
import {
  cancelFileTranscription,
  formatTauriError,
  onEvent,
  pickAudioFile,
  transcribeFile,
} from "../lib/tauri";
import type {
  FileTranscriptionRequest,
  FileTranscriptionResult,
} from "../types/app";
import type { FileTranscriptionProgressEvent } from "../types/events";

type Status = "idle" | "running" | "done" | "error";

// Lives in a store (not component state) so a running job's progress and
// result survive navigating away from the panel.
interface FileTranscriptionStore {
  path: string | null;
  status: Status;
  progress: FileTranscriptionProgressEvent | null;
  result: FileTranscriptionResult | null;
  error: string | null;
  pick: () => Promise<void>;
  start: (options: Omit<FileTranscriptionRequest, "path">) => Promise<void>;
  cancel: () => Promise<void>;
}

export const useFileTranscriptionStore = create<FileTranscriptionStore>(
  (set, get) => ({
    path: null,
    status: "idle",
    progress: null,
    result: null,
    error: null,

    pick: async () => {
      const path = await pickAudioFile();
      if (path) set({ path, status: "idle", result: null, error: null });
    },

    start: async (options) => {
      const { path, status } = get();
      if (!path || status === "running") return;
      set({ status: "running", progress: null, result: null, error: null });
      const unlisten = await onEvent<FileTranscriptionProgressEvent>(
        "file_transcription_progress",
        (progress) => set({ progress }),
      );
      try {
        const result = await transcribeFile({ ...options, path });
        set({ status: "done", result });
      } catch (err: unknown) {
        set({ status: "error", error: formatTauriError(err) });
      } finally {
        unlisten();
      }
    },

    cancel: async () => {
      await cancelFileTranscription();
    },
  }),
);
