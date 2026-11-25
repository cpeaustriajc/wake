type Theme = "dark" | "light";

interface ThemeInput {
  readonly stored: string | null;
  readonly systemDark: boolean;
}

const THEME_KEY = "keel-theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";
const THEME_LABELS: Readonly<Record<Theme, string>> = {
  dark: "Dark",
  light: "Light",
};

const systemTheme = (systemDark: boolean): Theme =>
  systemDark ? "dark" : "light";

const otherTheme = (theme: Theme): Theme =>
  theme === "dark" ? "light" : "dark";

export { DARK_QUERY, THEME_KEY, THEME_LABELS, otherTheme, systemTheme };
export type { Theme, ThemeInput };
