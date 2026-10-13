// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

export type ThemePreference = "system" | "light" | "dark";

export interface Preferences {
  shortcut: string;
  theme: ThemePreference;
}

const storageKey = "giftray.preferences.v1";
export const defaultPreferences: Preferences = {
  shortcut: "Ctrl+Alt+G",
  theme: "system",
};

export function readPreferences(storage: Storage = localStorage): Preferences {
  try {
    const raw = storage.getItem(storageKey);
    if (!raw) return defaultPreferences;
    const parsed: Partial<Preferences> = JSON.parse(raw);
    return {
      shortcut:
        typeof parsed.shortcut === "string"
          ? parsed.shortcut
          : defaultPreferences.shortcut,
      theme:
        parsed.theme === "light" ||
        parsed.theme === "dark" ||
        parsed.theme === "system"
          ? parsed.theme
          : defaultPreferences.theme,
    };
  } catch {
    return defaultPreferences;
  }
}

export function writePreferences(
  preferences: Preferences,
  storage: Storage = localStorage,
): void {
  storage.setItem(storageKey, JSON.stringify(preferences));
}
