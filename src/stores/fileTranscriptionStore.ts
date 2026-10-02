import { create } from "zustand";
import {
  cancelFileTranscription,
  formatTauriError,
  onEvent,
  pickAudioFile,
  transcribeFile,
} from "../lib/tauri";
import { useHistoryStore } from "./historyStore";
import type {
  FileTranscriptionRequest,
  FileTranscriptionResult,
} from "../types/app";
import type { FileTranscriptionProgressEvent } from "../types/events";

type Status = "idle" | "running" | "cancelling" | "done" | "error";
export type FileJobOptions = Omit<FileTranscriptionRequest, "path">;

const CANCELLED_CODE = "Cancelled";

function errorCode(err: unknown): string | null {
  if (err && typeof err === "object" && "code" in err && typeof err.code === "string") {
    return err.code;
  }
  return null;
}

// Lives in a store (not component state) so a running job's options,
// progress, and result survive navigating away from the panel.
interface FileTranscriptionStore {
  path: string | null;
  /** null until the panel seeds it from the user's settings. */
  options: FileJobOptions | null;
  picking: boolean;
  status: Status;
  progress: FileTranscriptionProgressEvent | null;
  result: FileTranscriptionResult | null;
  error: string | null;
  /** True after a user-initiated cancel, so the panel can say so neutrally. */
  cancelled: boolean;
  setOptions: (patch: Partial<FileJobOptions>) => void;
  initOptions: (defaults: FileJobOptions) => void;
  pick: () => Promise<void>;
  start: () => Promise<void>;
  cancel: () => Promise<void>;
}

const isBusy = (status: Status) => status === "running" || status === "cancelling";

export const useFileTranscriptionStore = create<FileTranscriptionStore>(
  (set, get) => ({
    path: null,
    options: null,
    picking: false,
    status: "idle",
    progress: null,
    result: null,
    error: null,
    cancelled: false,

    initOptions: (defaults) => {
      if (!get().options) set({ options: defaults });
    },

    setOptions: (patch) => {
      const { options, status } = get();
      if (!options || isBusy(status)) return;
      set({ options: { ...options, ...patch } });
    },

    pick: async () => {
      const { picking, status } = get();
      if (picking || isBusy(status)) return;
      set({ picking: true });
      try {
        const path = await pickAudioFile();
        if (path) {
          set({ path, status: "idle", result: null, error: null, cancelled: false });
        }
      } finally {
        set({ picking: false });
      }
    },

    start: async () => {
      const { path, options, status, picking } = get();
      if (!path || !options || picking || isBusy(status)) return;
      set({ status: "running", progress: null, result: null, error: null, cancelled: false });
      const unlisten = await onEvent<FileTranscriptionProgressEvent>(
        "file_transcription_progress",
        (progress) => set({ progress }),
      );
      try {
        const result = await transcribeFile({ ...options, path });
        set({ status: "done", result });
        void useHistoryStore.getState().load();
      } catch (err: unknown) {
        if (errorCode(err) === CANCELLED_CODE) {
          set({ status: "idle", cancelled: true });
        } else {
          set({ status: "error", error: formatTauriError(err) });
        }
      } finally {
        unlisten();
      }
    },

    cancel: async () => {
      if (get().status !== "running") return;
      set({ status: "cancelling" });
      await cancelFileTranscription();
    },
  }),
);
