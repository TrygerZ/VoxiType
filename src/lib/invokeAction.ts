import { toast } from "../components/ui/Toast";
import { formatTauriError } from "./tauri";

/**
 * Wraps an async IPC call, formats any rejected error, and displays an error toast.
 * Returns true if the action succeeded, false if an error occurred.
 */
export async function invokeAction<T>(
  fn: () => Promise<T>,
  onError?: (err: unknown) => void,
): Promise<boolean> {
  try {
    await fn();
    return true;
  } catch (err: unknown) {
    const message = formatTauriError(err);
    toast(message, "error");
    onError?.(err);
    return false;
  }
}
