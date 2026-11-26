import { useEffect, useRef, useState } from "react";
import { onEvent } from "../../lib/tauri";
import type { AudioLevelEvent } from "../../types/events";

interface WaveformProps {
  active: boolean;
  barCount?: number;
  barClassName?: string;
}

const STATIC_WAVE = [0.15, 0.25, 0.45, 0.65, 0.8, 0.65, 0.45, 0.35, 0.55, 0.75, 0.65, 0.45, 0.25, 0.15];

function buildInitialWave(barCount: number): number[] {
  if (barCount === STATIC_WAVE.length) return STATIC_WAVE;
  return Array.from({ length: barCount }, (_, index) => {
    const mappedIndex = Math.round(
      (index / Math.max(1, barCount - 1)) * (STATIC_WAVE.length - 1),
    );
    return STATIC_WAVE[mappedIndex] ?? 0.25;
  });
}

const boostLevel = (lvl: number): number => {
  if (lvl <= 0) return 0;
  // Boost quiet speech (0.01-0.05) using a power curve (x^0.4 * 2) to scale to visual height
  return Math.min(1, Math.pow(lvl, 0.4) * 2);
};

const nextBarValue = (lvl: number): number => {
  const jitter = 0.75 + Math.random() * 0.5;
  return Math.max(0.08, Math.min(1, boostLevel(lvl) * jitter));
};

function useWaveformBars(active: boolean, barCount: number): number[] {
  const [bars, setBars] = useState<number[]>(() => buildInitialWave(barCount));
  const levelRef = useRef(0);

  useEffect(() => {
    if (!active) {
      setBars(buildInitialWave(barCount));
      levelRef.current = 0;
      return;
    }
    setBars((prev) =>
      prev.length === barCount ? prev : buildInitialWave(barCount),
    );
    const unlistenPromise = onEvent<AudioLevelEvent>("audio_level", (p) => {
      levelRef.current = p.level;
    });
    const id = setInterval(() => {
      setBars((prev) => [...prev.slice(1), nextBarValue(levelRef.current)]);
    }, 50);
    return () => {
      clearInterval(id);
      void unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [active, barCount]);

  return bars;
}

export function Waveform({
  active,
  barCount = STATIC_WAVE.length,
  barClassName,
}: WaveformProps) {
  const bars = useWaveformBars(active, barCount);

  return (
    <div className="flex h-5 flex-1 items-center gap-[2px]" aria-hidden>
      {/* Index key is intentional: bars are stateless spans that slide left. */}
      {bars.map((v, i) => (
        <span
          key={i}
          className={`vx-wave-bar flex-1 rounded-full transition-[height] duration-[50ms] ${barClassName || "bg-vx-accent-ember"}`}
          style={{ height: `${Math.max(12, v * 100)}%` }}
        />
      ))}
    </div>
  );
}
