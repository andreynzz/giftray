// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import {
  failedPreviewTechnicalState,
  failedSelectionTechnicalState,
  initialImportTechnicalState,
  loadedImportTechnicalState,
  receivedImportTechnicalState,
  selectedImportTechnicalState,
} from "./import-state";

describe("import technical state", () => {
  it("records an accepted URL through selection and preview load", () => {
    const received = receivedImportTechnicalState({
      candidates: [
        { kind: "url", origin: "text/plain", url: "https://example.test/reaction.gif" },
      ],
      rejectedCount: 0,
    });

    expect(received).toMatchObject({
      input: "received",
      urlReceived: true,
      url: "accepted",
      selection: "pending",
    });
    expect(loadedImportTechnicalState(selectedImportTechnicalState(received))).toMatchObject({
      selection: "selected",
      preview: "loaded",
    });
  });

  it("distinguishes rejected URL input, selection failure and preview failure", () => {
    const received = receivedImportTechnicalState({ candidates: [], rejectedCount: 1 });
    expect(received.urlReceived).toBe(true);
    expect(received.url).toBe("rejected");
    expect(failedSelectionTechnicalState(received).selection).toBe("failed");
    expect(failedPreviewTechnicalState(initialImportTechnicalState).preview).toBe("failed");
  });
});
