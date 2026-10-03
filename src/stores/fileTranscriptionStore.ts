import { create } from "zustand";
import {
  cancelFileTranscription,
  exportTranscripts,
  formatTauriError,
  onEvent,
  pickAudioFiles,
  pickExportDirectory,
  transcribeFile,
} from "../lib/tauri";
import { useHistoryStore } from "./historyStore";
import type {
  FileTranscriptionRequest,
  FileTranscriptionResult,
  TranscriptExportFormat,
} from "../types/app";
import type { FileTranscriptionProgressEvent } from "../types/events";

type BatchStatus = "idle" | "running" | "cancelling";
export type ItemStatus = "queued" | "running" | "done" | "error" | "cancelled";
export type FileJobOptions = Omit<FileTranscriptionRequest, "path">;

export interface FileItem {
  path: string;
  status: ItemStatus;
  progress: FileTranscriptionProgressEvent | null;
  result: FileTranscriptionResult | null;
  error: string | null;
  /** Epoch ms; drive the per-file timer. */
  startedAt: number | null;
  finishedAt: number | null;
}

const CANCELLED_CODE = "Cancelled";

function errorCode(err: unknown): string | null {
  if (err && typeof err === "object" && "code" in err && typeof err.code === "string") {
    return err.code;
  }
  return null;
}

const queuedItem = (path: string): FileItem => ({
  path,
  status: "queued",
  progress: null,
  result: null,
  error: null,
  startedAt: null,
  finishedAt: null,
});

// Lives in a store (not component state) so a running batch's options,
// progress, and results survive navigating away from the panel.
interface FileTranscriptionStore {
  items: FileItem[];
  /** null until the panel seeds it from the user's settings. */
  options: FileJobOptions | null;
  picking: boolean;
  status: BatchStatus;
  /** True after a user-initiated cancel, so the panel can say so neutrally. */
  cancelled: boolean;
  exportFormat: TranscriptExportFormat;
  exporting: boolean;
  setOptions: (patch: Partial<FileJobOptions>) => void;
  initOptions: (defaults: FileJobOptions) => void;
  setExportFormat: (format: TranscriptExportFormat) => void;
  pick: () => Promise<void>;
  remove: (path: string) => void;
  start: () => Promise<void>;
  cancel: () => Promise<void>;
  /** Exports the given finished files; resolves to the written paths, or null if the folder dialog was closed. */
  exportFiles: (paths: string[]) => Promise<string[] | null>;
}

const isBusy = (status: BatchStatus) => status !== "idle";

export const useFileTranscriptionStore = create<FileTranscriptionStore>(
  (set, get) => {
    const patchItem = (path: string, patch: Partial<FileItem>) =>
      set({ items: get().items.map((item) => (item.path === path ? { ...item, ...patch } : item)) });

    /** Runs one file; returns false when the batch should stop. */
    const runItem = async (path: string, options: FileJobOptions): Promise<boolean> => {
      patchItem(path, { status: "running", progress: null, error: null, startedAt: Date.now(), finishedAt: null });
      try {
        const result = await transcribeFile({ ...options, path });
        patchItem(path, { status: "done", result, finishedAt: Date.now() });
        return true;
      } catch (err: unknown) {
        const wasCancel = errorCode(err) === CANCELLED_CODE;
        patchItem(path, {
          status: wasCancel ? "cancelled" : "error",
          error: wasCancel ? null : formatTauriError(err),
          finishedAt: Date.now(),
        });
        return !wasCancel;
      }
    };

    return {
      items: [],
      options: null,
      picking: false,
      status: "idle",
      cancelled: false,
      exportFormat: "txt",
      exporting: false,

      initOptions: (defaults) => {
        if (!get().options) set({ options: defaults });
      },

      setOptions: (patch) => {
        const { options, status } = get();
        if (!options || isBusy(status)) return;
        set({ options: { ...options, ...patch } });
      },

      setExportFormat: (exportFormat) => set({ exportFormat }),

      // A new pick replaces the queue: the backend only accepts paths from
      // the latest dialog result.
      pick: async () => {
        const { picking, status } = get();
        if (picking || isBusy(status)) return;
        set({ picking: true });
        try {
          const paths = await pickAudioFiles();
          if (paths.length > 0) {
            set({ items: paths.map(queuedItem), cancelled: false });
          }
        } finally {
          set({ picking: false });
        }
      },

      remove: (path) => {
        if (isBusy(get().status)) return;
        set({ items: get().items.filter((item) => item.path !== path) });
      },

      start: async () => {
        const { options, status, picking, items } = get();
        const pending = items.filter((item) => item.status !== "done").map((item) => item.path);
        if (!options || picking || isBusy(status) || pending.length === 0) return;
        set({ status: "running", cancelled: false });
        const unlisten = await onEvent<FileTranscriptionProgressEvent>(
          "file_transcription_progress",
          (progress) => {
            const running = get().items.find((item) => item.status === "running");
            if (running) patchItem(running.path, { progress });
          },
        );
        let stoppedByCancel = false;
        try {
          for (const path of pending) {
            // Checked before each file: the backend clears its cancel flag
            // when a new job begins, so a cancel between files must stop here.
            if (get().status === "cancelling" || !(await runItem(path, options))) {
              stoppedByCancel = true;
              break;
            }
          }
        } finally {
          unlisten();
          set({ status: "idle", cancelled: stoppedByCancel || get().status === "cancelling" });
          void useHistoryStore.getState().load();
        }
      },

      cancel: async () => {
        if (get().status !== "running") return;
        set({ status: "cancelling" });
        await cancelFileTranscription();
      },

      exportFiles: async (paths) => {
        const { exporting, exportFormat, items } = get();
        const ready = items.filter((item) => paths.includes(item.path) && item.result);
        if (exporting || ready.length === 0) return null;
        set({ exporting: true });
        try {
          const directory = await pickExportDirectory();
          if (!directory) return null;
          return await exportTranscripts(
            directory,
            exportFormat,
            ready.map((item) => ({ source_path: item.path, text: item.result?.text ?? "" })),
          );
        } finally {
          set({ exporting: false });
        }
      },
    };
  },
);
