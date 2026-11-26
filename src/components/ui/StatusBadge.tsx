import type { ComponentType, ReactNode } from "react";
import { AlertTriangle, AudioLines, Check, XCircle } from "lucide-react";

export type StatusBadgeTone =
  | "neutral"
  | "ember"
  | "success"
  | "warning"
  | "error";

export interface StatusBadgeProps {
  tone?: StatusBadgeTone;
  label?: string;
  children?: ReactNode;
  className?: string;
}

type SemanticTone = Exclude<StatusBadgeTone, "neutral">;

interface SemanticToneStyle {
  Icon: ComponentType<{
    className?: string;
    strokeWidth?: number;
    "aria-hidden"?: boolean | "true" | "false";
  }>;
  iconClass: string;
  textClass: string;
}

const semanticStyles: Record<SemanticTone, SemanticToneStyle> = {
  success: {
    Icon: Check,
    iconClass: "text-vx-semantic-success",
    textClass: "text-vx-text-secondary",
  },
  ember: {
    Icon: AudioLines,
    iconClass: "text-vx-accent-ember",
    textClass: "text-vx-text-primary",
  },
  warning: {
    Icon: AlertTriangle,
    iconClass: "text-vx-semantic-warning",
    textClass: "text-vx-text-secondary",
  },
  error: {
    Icon: XCircle,
    iconClass: "text-vx-semantic-error",
    textClass: "text-vx-semantic-error",
  },
};

export function StatusBadge({
  tone = "neutral",
  label,
  children,
  className = "",
}: StatusBadgeProps) {
  const content = label ?? children;

  if (tone === "neutral") {
    return (
      <span
        className={`inline-flex items-center rounded-[4px] bg-vx-bg-subtle px-[7px] py-[3px] text-xs font-normal leading-4 text-vx-text-secondary ${className}`}
      >
        {content}
      </span>
    );
  }

  const { Icon, iconClass, textClass } = semanticStyles[tone];
  return (
    <span
      className={`inline-flex items-center gap-[5px] py-0.5 text-xs font-normal leading-4 ${textClass} ${className}`}
    >
      <Icon
        className={`h-[13px] w-[13px] shrink-0 ${iconClass}`}
        strokeWidth={1.75}
        aria-hidden="true"
      />
      {content}
    </span>
  );
}
