import { useEffect } from "react";

import { DARK_QUERY, THEME_KEY } from "#/renderer/theme/theme-model.ts";
import {
  usePickedTheme,
  useTheme,
  useThemeSend,
} from "#/renderer/window/window-provider.tsx";

const ThemeAttribute = (): null => {
  const theme = useTheme();
  const picked = usePickedTheme();
  const send = useThemeSend();

  useEffect(() => {
    document.documentElement.dataset["theme"] = theme;
  }, [theme]);

  useEffect(() => {
    if (picked !== null) {
      localStorage.setItem(THEME_KEY, picked);
    }
  }, [picked]);

  useEffect(() => {
    const media = globalThis.matchMedia(DARK_QUERY);
    const follow = (): void => {
      send({ dark: media.matches, type: "system.changed" });
    };
    media.addEventListener("change", follow);
    return (): void => {
      media.removeEventListener("change", follow);
    };
  }, [send]);

  return null;
};

export { ThemeAttribute };
