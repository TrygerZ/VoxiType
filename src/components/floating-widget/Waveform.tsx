import { useEffect, useRef, useState } from "react";
import { onEvent } from "../../lib/tauri";
import type { AudioLevelEvent } from "../../types/events";

interface WaveformProps {
  level?: number;
  active: boolean;
  barClassName?: string;
}

const STATIC_WAVE = [0.15, 0.25, 0.45, 0.65, 0.8, 0.65, 0.45, 0.35, 0.55, 0.75, 0.65, 0.45, 0.25, 0.15];

const boostLevel = (lvl: number): number => {
  if (lvl <= 0) return 0;
  // Boost quiet speech (0.01-0.05) using a power curve (x^0.4 * 2) to scale to visual height
  return Math.min(1, Math.pow(lvl, 0.4) * 2);
};

function useWaveformBars(active: boolean, level?: number): number[] {
  const [bars, setBars] = useState<number[]>(STATIC_WAVE);
  const levelRef = useRef(level ?? 0);

  useEffect(() => {
    if (level !== undefined) levelRef.current = level;
  }, [level]);

  useEffect(() => {
    if (!active) {
      setBars(STATIC_WAVE);
      levelRef.current = 0;
      return;
    }
    let unlisten: (() => void) | undefined;
    void onEvent<AudioLevelEvent>("audio_level", (p) => {
      levelRef.current = p.level;
    }).then((fn) => {
      unlisten = fn;
    });
    const id = setInterval(() => {
      setBars((prev) => {
        const next = prev.slice(1);
        const jitter = 0.75 + Math.random() * 0.5;
        const boosted = boostLevel(levelRef.current);
        next.push(Math.max(0.08, Math.min(1, boosted * jitter)));
        return next;
      });
    }, 50);
    return () => {
      clearInterval(id);
      unlisten?.();
    };
  }, [active]);

  return bars;
}

export function Waveform({ level, active, barClassName }: WaveformProps) {
  const bars = useWaveformBars(active, level);

  return (
    <div className="flex h-5 flex-1 items-center gap-[2px]" aria-hidden>
      {/* ponytail: index key is intentional - bars are stateless identical spans,
          stable IDs would fight the sliding-window animation */}
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
