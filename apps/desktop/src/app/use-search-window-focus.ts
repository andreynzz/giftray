// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect } from "react";
import type { RefObject } from "react";
import { listenForSearchWindowOpened } from "../platform/tauri/window";

export function useSearchWindowFocus<T extends HTMLElement>(
  inputRef: RefObject<T | null>,
): void {
  useEffect(() => {
    const focus = () => inputRef.current?.focus();
    focus();
    let unlisten: (() => void) | undefined;
    listenForSearchWindowOpened(focus)
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, [inputRef]);
}
