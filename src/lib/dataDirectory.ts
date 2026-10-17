import { errorCode, formatTauriError } from "./tauri";

type Translator = (key: string) => string;

/** True when the backend refused the folder because of its permissions. */
export function isAclRejection(error: unknown): boolean {
  return errorCode(error) === "DataDirectoryAcl";
}

export function formatDirectoryError(error: unknown, t: Translator): string {
  if (isAclRejection(error)) return t("data_directory.error_acl");
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
