// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it, vi } from "vitest";
import { createTemporaryUrl, releaseTemporaryUrl } from "./temporary-url";

describe("temporary object URLs", () => {
  it("creates and releases a temporary URL", () => {
    const api = {
      createObjectURL: vi.fn(() => "blob:preview"),
      revokeObjectURL: vi.fn(),
    };
    const url = createTemporaryUrl(new File(["gif"], "reaction.gif"), api);
    releaseTemporaryUrl(url, api);

    expect(api.createObjectURL).toHaveBeenCalledOnce();
    expect(api.revokeObjectURL).toHaveBeenCalledWith("blob:preview");
  });

  it("does not revoke an absent URL", () => {
    const revokeObjectURL = vi.fn();
    releaseTemporaryUrl(undefined, { revokeObjectURL });
    expect(revokeObjectURL).not.toHaveBeenCalled();
  });
});
