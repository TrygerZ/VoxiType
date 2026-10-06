export interface DebouncedFn<Args extends unknown[]> {
  (...args: Args): void;
  cancel: () => void;
  flush: () => void;
}

export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => unknown,
  delay: number,
): DebouncedFn<Args> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pendingArgs: Args | null = null;

  const invoke = (args: Args) => {
    timer = null;
    pendingArgs = null;
    fn(...args);
  };

  const debounced = (...args: Args) => {
    if (timer) clearTimeout(timer);
    pendingArgs = args;
    timer = setTimeout(() => invoke(args), delay);
  };

  debounced.cancel = () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    pendingArgs = null;
  };

  // Persist a pending call immediately (e.g. on unmount) instead of dropping it.
  debounced.flush = () => {
    if (timer && pendingArgs) {
      clearTimeout(timer);
      invoke(pendingArgs);
    }
  };

  return debounced;
}
