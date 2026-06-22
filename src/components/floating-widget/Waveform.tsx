import { useEffect, useRef, useState } from "react";

interface WaveformProps {
  level: number;
  active: boolean;
}

const BARS = 28;

export function Waveform({ level, active }: WaveformProps) {
  // Keep a short rolling history so the bars look like a scrolling waveform
  // rather than every bar reacting identically.
  const [bars, setBars] = useState<number[]>(() => Array(BARS).fill(0.1));
  const levelRef = useRef(level);
  levelRef.current = level;

  useEffect(() => {
    if (!active) return;
    const id = setInterval(() => {
      setBars((prev) => {
        const next = prev.slice(1);
        // Add slight randomness around the current level for a lively feel.
        const jitter = 0.75 + Math.random() * 0.5;
        const value = Math.max(0.08, Math.min(1, levelRef.current * jitter));
        next.push(value);
        return next;
      });
    }, 60);
    return () => clearInterval(id);
  }, [active]);

  return (
    <div className="flex h-5 flex-1 items-center gap-[2px]" aria-hidden>
      {bars.map((v, i) => (
        <span
          key={i}
          className="flex-1 rounded-full bg-vx-accent transition-[height] duration-75"
          style={{ height: `${Math.max(12, v * 100)}%` }}
        />
      ))}
    </div>
  );
}
