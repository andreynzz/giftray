// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { MediaPreview } from "./components/MediaPreview";
import type { MediaPreviewState } from "./hooks/useMediaImport";

interface MediaImportProps {
  onClear: () => void;
  onLoad: (src: string) => void;
  onLoadError: (src: string) => void;
  preview: MediaPreviewState;
}

export function MediaImport(props: MediaImportProps) {
  return <MediaPreview {...props} />;
}

// The application composes this controller with the single input capture feature.
// eslint-disable-next-line react-refresh/only-export-components
export { useMediaImport } from "./hooks/useMediaImport";
