import { useEffect, useState } from "react";

interface WpmHalfRingProps {
  value: number;
  max?: number;
}

const CX = 68;
const CY = 72;
const RADIUS = 52;
const PERIMETER = Math.PI * RADIUS;
const TICK_COUNT = 7;

const CALIBRATION_TICKS = Array.from({ length: TICK_COUNT }, (_, i) => {
  const deg = 180 - i * 30;
  const rad = (deg * Math.PI) / 180;
  const rInner = 62;
  const isMajor = i % 3 === 0;
  const rOuter = isMajor ? 66.5 : 64.5;
  return {
    key: i,
    x1: (CX + rInner * Math.cos(rad)).toFixed(2),
    y1: (CY - rInner * Math.sin(rad)).toFixed(2),
    x2: (CX + rOuter * Math.cos(rad)).toFixed(2),
    y2: (CY - rOuter * Math.sin(rad)).toFixed(2),
    strokeWidth: isMajor ? 1.5 : 1,
  };
});

const prefersReducedMotion = (): boolean =>
  typeof window !== "undefined" &&
  Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)").matches);

export function WpmHalfRing({ value, max = 220 }: WpmHalfRingProps) {
  const targetProgress = Math.min(Math.max(value / max, 0), 1);
  const [animatedProgress, setAnimatedProgress] = useState<number>(() =>
    prefersReducedMotion() ? targetProgress : 0,
  );

  useEffect(() => {
    if (prefersReducedMotion()) {
      setAnimatedProgress(targetProgress);
      return;
    }

    if (typeof requestAnimationFrame !== "function") {
      setAnimatedProgress(targetProgress);
      return;
    }

    let rafId = 0;
    const startProgress = animatedProgress;
    const delta = targetProgress - startProgress;
    const duration = 600;
    const startTime = performance.now();

    const tick = (now: number) => {
      const elapsed = now - startTime;
      const t = Math.min(1, elapsed / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      setAnimatedProgress(startProgress + delta * eased);
      if (t < 1) {
        rafId = requestAnimationFrame(tick);
      }
    };

    rafId = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(rafId);
    };
    // Intentionally animate from current rendered progress toward new targetProgress
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [targetProgress]);

  const offset = PERIMETER * (1 - animatedProgress);
  const arc = `M ${CX - RADIUS} ${CY} A ${RADIUS} ${RADIUS} 0 0 1 ${CX + RADIUS} ${CY}`;

  return (
    <svg
      data-testid="wpm-half-ring"
      viewBox="0 0 136 88"
      className="h-[88px] w-[136px] shrink-0"
      fill="none"
      aria-hidden="true"
    >
      {/* Calibrated outer precision ticks */}
      {CALIBRATION_TICKS.map((tick) => (
        <line
          key={tick.key}
          x1={tick.x1}
          y1={tick.y1}
          x2={tick.x2}
          y2={tick.y2}
          stroke="var(--vx-border-strong)"
          strokeWidth={tick.strokeWidth}
          strokeLinecap="round"
        />
      ))}

      {/* Track arc */}
      <path
        d={arc}
        stroke="var(--vx-border-subtle)"
        strokeWidth={6}
        strokeLinecap="round"
      />

      {/* Ember progress arc */}
      <path
        d={arc}
        stroke="var(--vx-accent-ember)"
        strokeWidth={6}
        strokeLinecap="round"
        strokeDasharray={`${PERIMETER} ${PERIMETER * 5}`}
        strokeDashoffset={offset}
      />

      {/* Mono WPM readout */}
      <text
        x={CX}
        y="65"
        textAnchor="middle"
        fill="var(--vx-text-primary)"
        fontSize="26"
        fontWeight="700"
        fontFamily="var(--vx-font-mono)"
        letterSpacing="-0.02em"
      >
        {value}
      </text>

      {/* Unit */}
      <text
        x={CX}
        y="81"
        textAnchor="middle"
        fill="var(--vx-text-muted)"
        fontSize="10"
        fontWeight="500"
        fontFamily="var(--vx-font-family)"
        letterSpacing="0.14em"
      >
        WPM
      </text>
    </svg>
  );
}
