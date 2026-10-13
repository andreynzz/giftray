// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import type { NormalizedMediaInput } from "./media";

export interface ImportTechnicalState {
  input: "idle" | "received";
  preview: "idle" | "loading" | "loaded" | "failed";
  selection: "idle" | "pending" | "selected" | "failed";
  url: "not-applicable" | "received" | "accepted" | "rejected";
}

export const initialImportTechnicalState: ImportTechnicalState = {
  input: "idle",
  url: "not-applicable",
  selection: "idle",
  preview: "idle",
};

export function receivedImportTechnicalState(
  normalized: NormalizedMediaInput,
): ImportTechnicalState {
  const hasUrl = normalized.candidates.some((candidate) => candidate.kind === "url");
  const rejectedUrl = !hasUrl && normalized.rejectedCount > 0;

  return {
    input: "received",
    url: hasUrl ? "accepted" : rejectedUrl ? "rejected" : "not-applicable",
    selection: "pending",
    preview: "idle",
  };
}

export function selectedImportTechnicalState(
  state: ImportTechnicalState,
): ImportTechnicalState {
  return { ...state, selection: "selected", preview: "loading" };
}

export function failedSelectionTechnicalState(
  state: ImportTechnicalState,
): ImportTechnicalState {
  return { ...state, selection: "failed" };
}

export function loadedImportTechnicalState(
  state: ImportTechnicalState,
): ImportTechnicalState {
  return { ...state, preview: "loaded" };
}

export function failedPreviewTechnicalState(
  state: ImportTechnicalState,
): ImportTechnicalState {
  return { ...state, preview: "failed" };
}
