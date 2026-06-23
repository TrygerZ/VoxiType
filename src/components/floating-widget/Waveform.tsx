import { useEffect, useRef, useState } from "react";

interface WaveformProps {
  level: number;
  active: boolean;
  barClassName?: string;
}

const STATIC_WAVE = [0.15, 0.25, 0.45, 0.65, 0.8, 0.65, 0.45, 0.35, 0.55, 0.75, 0.65, 0.45, 0.25, 0.15];

const boostLevel = (lvl: number): number => {
  if (lvl <= 0) return 0;
  // Boost quiet speech (0.01-0.05) using a power curve (x^0.4 * 2) to scale to visual height
  return Math.min(1, Math.pow(lvl, 0.4) * 2);
};

export function Waveform({ level, active, barClassName }: WaveformProps) {
  // Keep a short rolling history so the bars look like a scrolling waveform
  // rather than every bar reacting identically.
  const [bars, setBars] = useState<number[]>(STATIC_WAVE);
  const levelRef = useRef(level);
  levelRef.current = level;

  useEffect(() => {
    if (!active) {
      setBars(STATIC_WAVE);
      return;
    }
    const id = setInterval(() => {
      setBars((prev) => {
        const next = prev.slice(1);
        // Add slight randomness around the current level for a lively feel.
        const jitter = 0.75 + Math.random() * 0.5;
        const boosted = boostLevel(levelRef.current);
        const value = Math.max(0.08, Math.min(1, boosted * jitter));
        next.push(value);
        return next;
      });
    }, 50); // Speed up tick to match backend 50ms polling interval
    return () => clearInterval(id);
  }, [active]);

  return (
    <div className="flex h-5 flex-1 items-center gap-[2px]" aria-hidden>
      {bars.map((v, i) => (
        <span
          key={i}
          className={`flex-1 rounded-full transition-[height] duration-[50ms] ${barClassName || "bg-vx-accent"}`}
          style={{ height: `${Math.max(12, v * 100)}%` }}
        />
      ))}
    </div>
  );
}
