// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it, vi } from "vitest";
import {
  registerAsyncNativeListener,
} from "./useInputDiagnostics";

describe("registerAsyncNativeListener", () => {
  it("removes a listener that resolves after cleanup", async () => {
    let resolveListener: ((dispose: () => void) => void) | undefined;
    const dispose = vi.fn();
    const register = vi.fn(
      () => new Promise<() => void>((resolve) => {
        resolveListener = resolve;
      }),
    );

    const cleanup = registerAsyncNativeListener(register, vi.fn(), vi.fn());
    cleanup();
    resolveListener?.(dispose);
    await Promise.resolve();

    expect(dispose).toHaveBeenCalledOnce();
  });

  it("does not report registration errors after cleanup", async () => {
    let rejectListener: ((reason?: unknown) => void) | undefined;
    const onError = vi.fn();
    const register = vi.fn(
      () => new Promise<() => void>((_resolve, reject) => {
        rejectListener = reject;
      }),
    );

    const cleanup = registerAsyncNativeListener(register, vi.fn(), onError);
    cleanup();
    rejectListener?.(new Error("listener unavailable"));
    await Promise.resolve();

    expect(onError).not.toHaveBeenCalled();
  });
});
