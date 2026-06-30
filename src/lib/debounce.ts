export interface DebouncedFn<F extends (...args: any[]) => any> {
  (...args: Parameters<F>): void;
  cancel: () => void;
}

export function debounce<F extends (...args: any[]) => any>(
  fn: F,
  delay: number,
): DebouncedFn<F> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const debounced = (...args: Parameters<F>) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      fn(...args);
      timer = null;
    }, delay);
  };
  debounced.cancel = () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  };
  return debounced;
}
