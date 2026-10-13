// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useEffect, useState } from "react";
import { teamsDarkTheme, teamsLightTheme } from "@fluentui/react-components";
import type { ThemePreference } from "../features/settings/preferences";

export function useSystemDarkMode(): boolean {
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  return systemDark;
}

export function resolveTheme(preference: ThemePreference, systemDark: boolean) {
  return preference === "dark" || (preference === "system" && systemDark)
    ? teamsDarkTheme
    : teamsLightTheme;
}
