import { formatTauriError } from "./tauri";

type Translator = (key: string) => string;

export function formatDirectoryError(error: unknown, t: Translator): string {
  const message = formatTauriError(error);
  const normalizedMessage = message.toLowerCase();

  if (normalizedMessage.includes("network drives are not supported")) {
    return t("data_directory.error_network");
  }
  if (normalizedMessage.includes("not writable")) {
    return t("data_directory.error_unwritable");
  }
  return message;
}
