export function Timer({ seconds }: { seconds: number }) {
  const mm = Math.floor(seconds / 60)
    .toString()
    .padStart(2, "0");
  const ss = Math.floor(seconds % 60)
    .toString()
    .padStart(2, "0");
  return <span className="font-mono text-sm tabular-nums">{`${mm}:${ss}`}</span>;
}
