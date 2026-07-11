import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { debounce } from "../lib/debounce";
import { formatTauriError } from "../lib/tauri";
import { toast } from "../components/ui/Toast";

// Debounce window before the API key is encrypted + written to SQLite, so the
// crypto write and "saved" toast don't fire on every keystroke.
const API_KEY_DEBOUNCE_MS = 600;

export interface DebouncedApiKey {
  localKey: string;
  onKeyChange: (value: string) => void;
}

// Holds the Groq API key in local state for responsive typing while debouncing
// the encrypted persist. storeValue resyncs the field when the shared key
// changes externally (initial load, or saved from another view).
export function useDebouncedApiKey(
  storeValue: string,
  update: (key: string, value: unknown) => Promise<void>,
  savedLabel: string,
): DebouncedApiKey {
  const [localKey, setLocalKey] = useState(storeValue);

  useEffect(() => {
    setLocalKey(storeValue);
  }, [storeValue]);

  const savedLabelRef = useRef(savedLabel);
  savedLabelRef.current = savedLabel;

  const persist = useCallback(
    async (value: string) => {
      try {
        await update("groq_api_key", value);
        if (value.trim()) toast(savedLabelRef.current);
      } catch (e: unknown) {
        toast(formatTauriError(e), "error");
      }
    },
    [update],
  );

  const debouncedPersist = useMemo(
    () => debounce(persist, API_KEY_DEBOUNCE_MS),
    [persist],
  );

  const onKeyChange = (value: string) => {
    setLocalKey(value);
    debouncedPersist(value);
  };

  return { localKey, onKeyChange };
}
