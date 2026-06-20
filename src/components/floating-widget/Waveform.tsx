interface WaveformProps {
  level: number;
  active: boolean;
}

const BARS = 24;

export function Waveform({ level, active }: WaveformProps) {
  return (
    <div className="flex h-8 items-center gap-0.5" aria-hidden>
      {Array.from({ length: BARS }).map((_, i) => {
        const center = (BARS - 1) / 2;
        const distance = Math.abs(i - center) / center;
        const scale = active
          ? Math.max(0.15, level * (1 - distance * 0.7) + 0.1)
          : 0.15;
        return (
          <span
            key={i}
            className="w-1 rounded-full bg-vx-accent transition-transform"
            style={{
              height: "100%",
              transform: `scaleY(${scale})`,
              transformOrigin: "center",
            }}
          />
        );
      })}
    </div>
  );
}
