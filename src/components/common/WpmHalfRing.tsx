interface WpmHalfRingProps {
  value: number;
  max?: number;
}

export function WpmHalfRing({ value, max = 400 }: WpmHalfRingProps) {
  const cx = 70, cy = 78, r = 58;
  const perimeter = Math.PI * r;
  const progress = Math.min(Math.max(value / max, 0), 1);
  const offset = perimeter * (1 - progress);
  const arc = `M ${cx - r} ${cy} A ${r} ${r} 0 0 1 ${cx + r} ${cy}`;

  return (
    <svg
      viewBox="0 0 140 110"
      className="w-full max-w-[140px] h-auto"
      fill="none"
      aria-hidden
    >
      {/* Track arc */}
      <path
        d={arc}
        stroke="var(--color-vx-border)"
        strokeWidth={6}
        strokeLinecap="round"
      />
      {/* Progress arc */}
      <path
        d={arc}
        stroke="var(--color-vx-accent)"
        strokeWidth={6}
        strokeLinecap="round"
        strokeDasharray={`${perimeter} ${perimeter * 5}`}
        strokeDashoffset={offset}
        className="motion-safe:transition-[stroke-dashoffset] motion-safe:duration-700 motion-safe:ease-out"
      />
      {/* Value */}
      <text
        x="70" y="82"
        textAnchor="middle"
        fill="var(--color-vx-text-primary)"
        fontSize="28"
        fontWeight="700"
        fontFamily="var(--vx-font-family)"
        letterSpacing="-0.02"
      >
        {value}
      </text>
      {/* Unit */}
      <text
        x="70" y="98"
        textAnchor="middle"
        fill="var(--color-vx-text-secondary)"
        fontSize="10"
        fontWeight="500"
        fontFamily="var(--vx-font-family)"
        letterSpacing="1.5"
      >
        WPM
      </text>
    </svg>
  );
}
