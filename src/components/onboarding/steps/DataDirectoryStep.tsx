import { useEffect, useState } from "react";
import {
  ArrowLeft,
  Check,
  ChevronRight,
  FolderOpen,
  RotateCw,
} from "lucide-react";

import {
  formatDirectoryError,
  isAclRejection,
} from "../../../lib/dataDirectory";
import {
  errorCode,
  formatTauriError,
  getDataDirectory,
  pickDataDirectory,
  restartApp,
  restrictDataDirectory,
  setDataDirectory,
} from "../../../lib/tauri";
import { useSettingsStore } from "../../../stores/settingsStore";
import { AclRestrictPrompt } from "../../ui/AclRestrictPrompt";
import { Button } from "../../ui/Button";
import { StepShell } from "../shared/StepShell";
import type { Step, TFunc } from "../types";

interface Props {
  step: Step;
  currentStepIdx: number;
  t: TFunc;
  onBack: () => void;
  onSkip: () => void;
  onContinue: () => void;
}

export function DataDirectoryStep(props: Props) {
  const { t } = props;
  const [activePath, setActivePath] = useState("");
  const [selectedPath, setSelectedPath] = useState("");
  const [pendingRestart, setPendingRestart] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmRestart, setConfirmRestart] = useState(false);
  const [aclRejected, setAclRejected] = useState(false);

  useEffect(() => {
    void getDataDirectory()
      .then((dirStatus) => {
        setActivePath(dirStatus.active);
        setPendingRestart(dirStatus.pending);
      })
      .catch((e: unknown) => setError(formatDirectoryError(e, t)));
  }, [t]);

  const choose = async () => {
    setBusy(true);
    try {
      setError("");
      const path = await pickDataDirectory();
      if (path) setSelectedPath(path);
    } catch (e: unknown) {
      setError(formatDirectoryError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    if (!selectedPath || busy) return;
    setBusy(true);
    try {
      setError("");
      setAclRejected(false);
      await setDataDirectory(selectedPath);
      setPendingRestart(selectedPath);
      setSelectedPath("");
      setStatus(t("data_directory.success"));
    } catch (e: unknown) {
      // A folder on another drive inherits write access for every local
      // account. Offer the fix instead of dead-ending the user.
      if (isAclRejection(e)) {
        setAclRejected(true);
        setError("");
      } else {
        setError(formatDirectoryError(e, t));
      }
    } finally {
      setBusy(false);
    }
  };

  const restrictAndApply = async () => {
    if (!selectedPath || busy) return;
    setBusy(true);
    try {
      setError("");
      await restrictDataDirectory(selectedPath);
      setAclRejected(false);
      setPendingRestart(selectedPath);
      setSelectedPath("");
      setStatus(t("data_directory.success"));
    } catch (e: unknown) {
      setAclRejected(false);
      setError(
        t("data_directory.acl_failed", { reason: formatTauriError(e) }),
      );
    } finally {
      setBusy(false);
    }
  };

  const handleRestart = async (force = false) => {
    if (busy) return;
    setBusy(true);
    try {
      setError("");
      await useSettingsStore
        .getState()
        .update("onboarding_resume_step", "hotkey");
      await restartApp(force);
      setConfirmRestart(false);
    } catch (e: unknown) {
      // The backend refuses a restart while work is active; ask first.
      if (errorCode(e) === "WorkInProgress") {
        setConfirmRestart(true);
      } else {
        setError(formatDirectoryError(e, t));
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <StepShell
      step={props.step}
      currentStepIdx={props.currentStepIdx}
      t={t}
      title={t("data_directory.title")}
      description={t("data_directory.body")}
      error={error}
      actions={
        <>
          <Button variant="ghost" size="lg" onClick={props.onBack}>
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.back")}
          </Button>
          <Button variant="ghost" size="lg" onClick={props.onSkip}>
            {t("data_directory.skip")}
          </Button>
          <Button variant="primary" size="lg" onClick={props.onContinue}>
            {t("data_directory.continue")}
            <ChevronRight className="h-4 w-4" />
          </Button>
        </>
      }
    >
      <div className="w-full max-w-lg rounded-[12px] border border-vx-border-subtle bg-vx-bg-surface p-5 text-left">
        <p className="text-xs font-medium text-vx-text-muted">
          {t("data_directory.active")}
        </p>
        <p className="mt-2 break-all rounded-[8px] border border-vx-border-subtle bg-vx-bg-subtle px-3 py-2 font-mono text-xs text-vx-text-primary">
          {activePath || t("data_directory.loading")}
        </p>

        {selectedPath && (
          <>
            <p className="mt-3 text-xs font-medium text-vx-text-muted">
              {t("data_directory.preview")}
            </p>
            <p className="mt-1.5 break-all rounded-[8px] border border-vx-accent-ember/30 bg-vx-accent-ember-soft px-3 py-2 font-mono text-xs text-vx-text-primary">
              {selectedPath}
            </p>
          </>
        )}

        <div className="mt-4 flex flex-wrap gap-2">
          <Button
            type="button"
            variant="secondary"
            disabled={busy}
            onClick={() => void choose()}
          >
            <FolderOpen className="h-4 w-4" />
            {t("data_directory.choose")}
          </Button>
          <Button
            type="button"
            variant="primary"
            disabled={!selectedPath || busy}
            onClick={() => void apply()}
          >
            <Check className="h-4 w-4" />
            {t("data_directory.apply")}
          </Button>
          {pendingRestart && !confirmRestart && (
            <Button
              type="button"
              variant="secondary"
              disabled={busy}
              onClick={() => void handleRestart()}
              data-testid="restart-app-button"
            >
              <RotateCw className="h-4 w-4" />
              {t("data_directory.apply_restart_resume")}
            </Button>
          )}
          {pendingRestart && confirmRestart && (
            <>
              <Button
                type="button"
                variant="danger"
                disabled={busy}
                onClick={() => void handleRestart(true)}
                data-testid="restart-app-confirm-button"
              >
                {t("data_directory.restart_confirm_btn")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                disabled={busy}
                onClick={() => setConfirmRestart(false)}
                data-testid="restart-app-cancel-button"
              >
                {t("data_directory.restart_cancel")}
              </Button>
            </>
          )}
        </div>

        {confirmRestart && (
          <p
            role="alert"
            className="mt-3 text-xs text-vx-semantic-error"
            data-testid="restart-app-confirm-message"
          >
            {t("data_directory.restart_confirm")}
          </p>
        )}

        {pendingRestart && (
          <div
            className="mt-3 rounded-[8px] border border-vx-accent-ember/30 bg-vx-accent-ember-soft px-3 py-2"
            data-testid="pending-data-directory"
          >
            <p className="break-all font-mono text-xs text-vx-accent-ember">
              {t("data_directory.pending_restart", { path: pendingRestart })}
            </p>
          </div>
        )}

        {aclRejected && (
          <AclRestrictPrompt
            message={t("data_directory.acl_confirm")}
            onConfirm={() => void restrictAndApply()}
            onCancel={() => {
              setAclRejected(false);
              setSelectedPath("");
            }}
            busy={busy}
            t={t}
          />
        )}

        <p className="mt-4 text-xs leading-relaxed text-vx-text-muted">
          {t("data_directory.acl_hint")}
        </p>

        {status && (
          <p role="status" className="mt-2.5 text-xs text-vx-semantic-success">
            {status}
          </p>
        )}
      </div>
    </StepShell>
  );
}
