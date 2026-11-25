type Theme = "dark" | "light";

const THEME_KEY = "keel-theme";

const themeOf = (picked: unknown, systemDark: boolean): Theme => {
  if (picked === "dark" || picked === "light") {
    return picked;
  }
  if (systemDark) {
    return "dark";
  }
  return "light";
};

const otherTheme = (theme: Theme): Theme => {
  if (theme === "dark") {
    return "light";
  }
  return "dark";
};

const THEME_LABELS: Readonly<Record<Theme, string>> = { dark: "Dark", light: "Light" };

export { THEME_KEY, THEME_LABELS, otherTheme, themeOf };
export type { Theme };
