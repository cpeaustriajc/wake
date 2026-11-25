import { assign, setup } from "xstate";
import type { SnapshotFrom } from "xstate";

import { systemTheme } from "#/renderer/theme/theme-model.ts";
import type { Theme, ThemeInput } from "#/renderer/theme/theme-model.ts";

interface ThemeContext {
  readonly stored: string | null;
  readonly systemDark: boolean;
}

type ThemeEvent =
  | { readonly dark: boolean; readonly type: "system.changed" }
  | { readonly type: "theme.toggled" };

const anyEvent = (): ThemeEvent => ({ type: "theme.toggled" });
const anyInput = (): ThemeInput => ({ stored: null, systemDark: false });
const anyContext = (): ThemeContext => anyInput();

const themeMachine = setup({
  actions: {
    followSystem: assign((_args, dark: boolean) => ({ systemDark: dark })),
  },
  guards: {
    storedDark: ({ context }) => context.stored === "dark",
    storedLight: ({ context }) => context.stored === "light",
    systemIsDark: ({ context }) => context.systemDark,
  },
  types: { context: anyContext(), events: anyEvent(), input: anyInput() },
}).createMachine({
  context: ({ input }) => input,
  id: "theme",
  initial: "starting",
  on: {
    "system.changed": {
      actions: { params: ({ event }) => event.dark, type: "followSystem" },
    },
  },
  states: {
    dark: { on: { "theme.toggled": "light" } },
    following: {
      on: {
        "theme.toggled": [
          { guard: "systemIsDark", target: "light" },
          { target: "dark" },
        ],
      },
    },
    light: { on: { "theme.toggled": "dark" } },
    starting: {
      always: [
        { guard: "storedDark", target: "dark" },
        { guard: "storedLight", target: "light" },
        { target: "following" },
      ],
    },
  },
});

type ThemeSnapshot = SnapshotFrom<typeof themeMachine>;

const themeIn = (snapshot: ThemeSnapshot): Theme => {
  if (snapshot.matches("dark")) {
    return "dark";
  }
  if (snapshot.matches("light")) {
    return "light";
  }
  return systemTheme(snapshot.context.systemDark);
};

const pickedIn = (snapshot: ThemeSnapshot): Theme | null => {
  if (snapshot.matches("dark")) {
    return "dark";
  }
  return snapshot.matches("light") ? "light" : null;
};

export { pickedIn, themeIn, themeMachine };
export type { ThemeSnapshot };
