// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { describe, expect, it } from "vitest";
import {
  defaultPreferences,
  readPreferences,
  writePreferences,
} from "./preferences";

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
}

describe("preferences", () => {
  it("usa valores seguros quando a preferência não existe", () => {
    expect(readPreferences(memoryStorage())).toEqual(defaultPreferences);
  });

  it("persiste atalho e tema", () => {
    const storage = memoryStorage();
    writePreferences({ shortcut: "Ctrl+Shift+G", theme: "dark" }, storage);
    expect(readPreferences(storage)).toEqual({
      shortcut: "Ctrl+Shift+G",
      theme: "dark",
    });
  });
});
