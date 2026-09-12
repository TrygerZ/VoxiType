import { useEffect, useState } from "react";
import {
  ArrowLeft,
  Check,
  ChevronRight,
  FolderOpen,
  HardDrive,
} from "lucide-react";

import {
  getDataDirectory,
  pickDataDirectory,
  setDataDirectory,
} from "../../../lib/tauri";
import { formatDirectoryError } from "../../../lib/dataDirectory";
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
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void getDataDirectory()
      .then((status) => setActivePath(status.active))
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
      await setDataDirectory(selectedPath);
      setSelectedPath("");
      setStatus(t("data_directory.success"));
    } catch (e: unknown) {
      setError(formatDirectoryError(e, t));
    } finally {
      setBusy(false);
    }
  };

  return (
    <StepShell
      step={props.step}
      currentStepIdx={props.currentStepIdx}
      t={t}
      icon={<HardDrive className="h-7 w-7" />}
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
      <div className="w-full max-w-xl rounded-xl border border-vx-border bg-vx-bg-secondary p-5 text-left shadow-vx-sm">
        <p className="text-xs font-medium text-vx-text-dim">
          {t("data_directory.active")}
        </p>
        <p className="mt-2 break-all rounded-lg bg-vx-bg-tertiary px-3 py-2 font-mono text-xs text-vx-text-primary">
          {activePath || t("data_directory.loading")}
        </p>
        {selectedPath && (
          <>
            <p className="mt-4 text-xs font-medium text-vx-text-dim">
              {t("data_directory.preview")}
            </p>
            <p className="mt-2 break-all rounded-lg bg-vx-accent-soft px-3 py-2 font-mono text-xs text-vx-text-primary">
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
        </div>
        <p className="mt-4 text-xs leading-relaxed text-vx-text-dim">
          {t("data_directory.restart")}
        </p>
        {status && (
          <p role="status" className="mt-3 text-xs text-vx-success">
            {status}
          </p>
        )}
      </div>
    </StepShell>
  );
}
