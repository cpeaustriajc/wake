import { THEME_KEY, otherTheme, themeOf } from "#/renderer/theme/theme-model.ts";
import { createEffect, createMemo, createSignal, onSettled } from "solid-js";
import type { Accessor } from "solid-js";
import type { Theme } from "#/renderer/theme/theme-model.ts";

interface ThemeControl {
  readonly theme: Accessor<Theme>;
  readonly toggle: () => void;
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

const createTheme = (): ThemeControl => {
  const media = globalThis.matchMedia(DARK_QUERY);
  const [picked, setPicked] = createSignal(localStorage.getItem(THEME_KEY));
  const [systemDark, setSystemDark] = createSignal(media.matches);
  const theme = createMemo(() => themeOf(picked(), systemDark()));
  onSettled(() => {
    const follow = (): void => {
      setSystemDark(media.matches);
    };
    media.addEventListener("change", follow);
    return (): void => {
      media.removeEventListener("change", follow);
    };
  });
  createEffect(theme, (value) => {
    document.documentElement.dataset["theme"] = value;
  });
  const toggle = (): void => {
    const next = otherTheme(theme());
    localStorage.setItem(THEME_KEY, next);
    setPicked(next);
  };
  return { theme, toggle };
};

export { createTheme };
