import { Button } from "./Button";

interface Props {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  busy: boolean;
  t: (key: string) => string;
}

/**
 * Inline confirmation shown after the backend refuses a folder because another
 * local account can write to it. Restricting rewrites the folder's DACL, which
 * is a change to something the app does not own, so the user agrees first.
 */
export function AclRestrictPrompt({
  message,
  onConfirm,
  onCancel,
  busy,
  t,
}: Props) {
  return (
    <div
      role="alert"
      className="mt-3 rounded-lg border border-vx-warning/40 bg-vx-warning/10 p-3"
      data-testid="acl-restrict-prompt"
    >
      <p className="text-xs leading-relaxed text-vx-text-primary">{message}</p>
      <div className="mt-3 flex flex-wrap gap-2">
        <Button
          type="button"
          size="sm"
          variant="primary"
          disabled={busy}
          onClick={onConfirm}
          data-testid="acl-restrict-confirm"
        >
          {t("data_directory.acl_apply")}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={onCancel}
          data-testid="acl-restrict-cancel"
        >
          {t("data_directory.acl_cancel")}
        </Button>
      </div>
    </div>
  );
}
