import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { downloadBlob } from "./downloadBlob";

describe("downloadBlob", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.restoreAllMocks();
    document.body.innerHTML = "";

    if (!URL.createObjectURL) {
      URL.createObjectURL = () => "blob:mock-url";
    }
    if (!URL.revokeObjectURL) {
      URL.revokeObjectURL = () => undefined;
    }

    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
    document.body.innerHTML = "";
  });

  it("appends anchor to DOM before click is invoked", () => {
    let wasConnected = false;
    let hadParentNode = false;

    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:mock-url");
    const appendChildSpy = vi.spyOn(document.body, "appendChild");

    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      function (this: HTMLAnchorElement) {
        wasConnected = this.isConnected;
        hadParentNode = document.body.contains(this);
      },
    );

    downloadBlob("test content", "sample.txt", "text/plain");

    expect(appendChildSpy).toHaveBeenCalledTimes(1);
    expect(wasConnected).toBe(true);
    expect(hadParentNode).toBe(true);
  });

  it("does not revoke object URL synchronously on the same tick, revokes after macro-task", () => {
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:mock-url");
    const revokeSpy = vi.spyOn(URL, "revokeObjectURL");

    downloadBlob("test content", "sample.txt", "text/plain");

    expect(revokeSpy).not.toHaveBeenCalled();

    vi.runAllTimers();

    expect(revokeSpy).toHaveBeenCalledTimes(1);
    expect(revokeSpy).toHaveBeenCalledWith("blob:mock-url");
  });

  it("cleans up the anchor element from the DOM after timer runs", () => {
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:mock-url");

    downloadBlob("test content", "sample.txt", "text/plain");

    const anchorBefore = document.body.querySelector("a");
    expect(anchorBefore).not.toBeNull();
    expect(anchorBefore?.download).toBe("sample.txt");

    vi.runAllTimers();

    const anchorAfter = document.body.querySelector("a");
    expect(anchorAfter).toBeNull();
  });

  it("creates Blob with specified MIME type and sets anchor attributes correctly", () => {
    const captured: {
      blob: Blob | null;
      anchor: HTMLAnchorElement | null;
    } = {
      blob: null,
      anchor: null,
    };

    vi.spyOn(URL, "createObjectURL").mockImplementation((obj: Blob | MediaSource) => {
      if (obj instanceof Blob) {
        captured.blob = obj;
      }
      return "blob:custom-url";
    });

    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      function (this: HTMLAnchorElement) {
        captured.anchor = this;
      },
    );

    downloadBlob("{\"key\":\"val\"}", "voxitype-dictionary.json", "application/json");

    expect(captured.blob).toBeInstanceOf(Blob);
    if (captured.blob) {
      expect(captured.blob.type).toBe("application/json");
    }
    expect(captured.anchor).toBeInstanceOf(HTMLAnchorElement);
    if (captured.anchor) {
      expect(captured.anchor.download).toBe("voxitype-dictionary.json");
      expect(captured.anchor.href).toBe("blob:custom-url");
    }
  });

  it("handles CSV format correctly", () => {
    const captured: {
      blob: Blob | null;
    } = {
      blob: null,
    };

    vi.spyOn(URL, "createObjectURL").mockImplementation((obj: Blob | MediaSource) => {
      if (obj instanceof Blob) {
        captured.blob = obj;
      }
      return "blob:custom-csv-url";
    });

    downloadBlob("col1,col2\nval1,val2", "voxitype-history.csv", "text/csv");

    expect(captured.blob).toBeInstanceOf(Blob);
    if (captured.blob) {
      expect(captured.blob.type).toBe("text/csv");
    }
    const anchor = document.body.querySelector("a");
    expect(anchor).toBeInstanceOf(HTMLAnchorElement);
    if (anchor) {
      expect(anchor.download).toBe("voxitype-history.csv");
      expect(anchor.href).toBe("blob:custom-csv-url");
    }
  });
});
