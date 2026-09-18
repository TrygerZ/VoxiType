// ponytail: in-memory DOM blob download; upgrade to tauri save dialog if exporting large multi-megabyte datasets
const CLEANUP_DELAY_MS = 0;

/**
 * Initiates a browser download for given content via an ephemeral anchor element.
 * Attaches to DOM and delays cleanup to allow WebView2 to asynchronously commit the download.
 */
export function downloadBlob(
  content: string,
  filename: string,
  mimeType: string,
): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();

  setTimeout(() => {
    anchor.remove();
    URL.revokeObjectURL(url);
  }, CLEANUP_DELAY_MS);
}
