import { defineConfig } from "oxlint";
import core from "ultracite/oxlint/core";
import react from "ultracite/oxlint/react";
import vitest from "ultracite/oxlint/vitest";

interface PathBan {
  importNames?: string[];
  message: string;
  name: string;
}

interface PatternBan {
  message: string;
  regex: string;
}

const SOCKETS = "Only src/main opens sockets.";
const STATE = "Client state lives in the XState machines.";
const COMPILER = "React Compiler memoizes components and handlers.";
const EFFECTS =
  "Only the provider, the theme attribute and the terminal host run effects.";
const MACHINES = "Only *-machine.ts and the provider import XState.";
const VIEWS = "Only views import React, UI SDK and StyleX.";
const PIPE = "The main process and the preload hold no view code.";
const BRIDGE = "Views reach the daemon only through the bridge.";

const net: readonly PathBan[] = [
  { message: SOCKETS, name: "node:net" },
  { message: SOCKETS, name: "net" },
];

const reactState: readonly PathBan[] = [
  {
    importNames: ["useState", "useReducer", "useSyncExternalStore"],
    message: STATE,
    name: "react",
  },
  {
    importNames: ["memo", "useCallback", "useMemo"],
    message: COMPILER,
    name: "react",
  },
];

const reactEffects: readonly PathBan[] = [
  {
    importNames: ["useEffect", "useLayoutEffect", "useInsertionEffect"],
    message: EFFECTS,
    name: "react",
  },
];

const reactAll: readonly PathBan[] = [
  { message: VIEWS, name: "react" },
  { message: VIEWS, name: "react-dom" },
  { message: VIEWS, name: "react-dom/client" },
  { message: VIEWS, name: "@stylexjs/stylex" },
];

const xstate: readonly PathBan[] = [{ message: MACHINES, name: "xstate" }];
const xstateReact: readonly PathBan[] = [
  {
    message: "Only the provider imports @xstate/react.",
    name: "@xstate/react",
  },
];
const electron: readonly PathBan[] = [
  { message: "Only src/main and src/preload use Electron.", name: "electron" },
];
const effect: readonly PathBan[] = [
  { message: "The preload runs no library code.", name: "effect" },
];

const earnestModels: PatternBan = {
  message: "Earnest models are read by the checker only.",
  regex: "\\.model(\\.ts)?$",
};
const stylesheets: PatternBan = {
  message: "Only the entry imports CSS, and only Keel's two files.",
  regex: "\\.css$",
};
const xterm: PatternBan = {
  message: "Only the terminal host draws with xterm.",
  regex: "^@xterm/[^/]+$",
};
const node: PatternBan = {
  message: "Only src/main and the tests use Node.",
  regex: "^node:",
};
const uiSdk: PatternBan = { message: VIEWS, regex: "^@ui-sdk/" };
const viewFiles: PatternBan = {
  message: "Models and machines import no view.",
  regex: "\\.tsx$",
};
const mainAndPreload: PatternBan = {
  message: BRIDGE,
  regex: "^#/(main|preload)/",
};
const rendererCode: PatternBan = { message: PIPE, regex: "^#/renderer/" };
const outsideShared: PatternBan = {
  message: "src/shared imports only src/shared and effect.",
  regex: "^(\\.|node:|#/(main|preload|renderer)/)",
};

const bans = (
  paths: readonly PathBan[],
  patterns: readonly PatternBan[]
): ["error", { paths: PathBan[]; patterns: PatternBan[] }] => [
  "error",
  { paths: [...paths], patterns: [...patterns] },
];

const everywhere: readonly PatternBan[] = [earnestModels, stylesheets, xterm];
const viewPaths = [...net, ...electron, ...reactState];
const viewPatterns = [...everywhere, node, mainAndPreload];
const purePaths = [...viewPaths, ...reactAll, ...xstateReact];
const purePatterns = [...viewPatterns, uiSdk, viewFiles];
const pipePaths = [...reactAll, ...xstate, ...xstateReact];

const PROVIDER = "src/renderer/window/window-provider.tsx";
const THEME_ATTRIBUTE = "src/renderer/theme/theme-attribute.tsx";
const ENTRY = "src/renderer/main.tsx";
const TERMINAL_HOST = "src/renderer/workspace/terminal-host.tsx";
const BROWSER_GLOBALS = [
  "window",
  "document",
  "navigator",
  "localStorage",
  "sessionStorage",
];
const NODE_GLOBALS = [
  "process",
  "Buffer",
  "require",
  "__dirname",
  "__filename",
];

export default defineConfig({
  extends: [core, react, vitest],
  ignorePatterns: [
    "lint/**",
    "out/**",
    "src/renderer/keel/*.css",
    "src/**/*.model.ts",
  ],
  jsPlugins: [
    "eslint-plugin-no-comments",
    "@stylexjs/eslint-plugin",
    "./lint/wake-styles.mjs",
  ],
  options: {
    denyWarnings: true,
    reportUnusedDisableDirectives: "error",
    respectEslintDisableDirectives: false,
    typeAware: true,
  },
  overrides: [
    {
      files: ["src/main/daemon-client.ts"],
      rules: { "unicorn/throw-new-error": "off" },
    },
    {
      files: ["src/main/**"],
      rules: {
        "no-restricted-globals": ["error", ...BROWSER_GLOBALS],
        "no-restricted-imports": bans(pipePaths, [
          ...everywhere,
          uiSdk,
          rendererCode,
        ]),
      },
    },
    {
      files: ["src/preload/**"],
      rules: {
        "no-restricted-globals": ["error", ...NODE_GLOBALS],
        "no-restricted-imports": bans(
          [...pipePaths, ...net, ...effect],
          [
            ...everywhere,
            uiSdk,
            node,
            rendererCode,
            { message: PIPE, regex: "^#/main/" },
          ]
        ),
      },
    },
    {
      files: ["src/shared/**"],
      rules: {
        "no-restricted-globals": ["error", ...NODE_GLOBALS, ...BROWSER_GLOBALS],
        "no-restricted-imports": bans(
          [...pipePaths, ...net, ...electron],
          [...everywhere, uiSdk, outsideShared]
        ),
      },
    },
    {
      files: ["src/renderer/**"],
      rules: {
        "no-restricted-globals": ["error", ...NODE_GLOBALS],
        "no-restricted-imports": bans(
          [...viewPaths, ...reactEffects, ...xstate, ...xstateReact],
          viewPatterns
        ),
      },
    },
    {
      files: ["src/renderer/**/*-model.ts"],
      rules: {
        "no-restricted-globals": ["error", ...NODE_GLOBALS, ...BROWSER_GLOBALS],
        "no-restricted-imports": bans([...purePaths, ...xstate], purePatterns),
      },
    },
    {
      files: ["src/renderer/**/*-machine.ts"],
      rules: {
        "no-restricted-globals": ["error", ...NODE_GLOBALS, ...BROWSER_GLOBALS],
        "no-restricted-imports": bans(purePaths, purePatterns),
      },
    },
    {
      files: [PROVIDER],
      rules: { "no-restricted-imports": bans(viewPaths, viewPatterns) },
    },
    {
      files: [THEME_ATTRIBUTE],
      rules: {
        "no-restricted-imports": bans(
          [...viewPaths, ...xstate, ...xstateReact],
          viewPatterns
        ),
      },
    },
    {
      files: [TERMINAL_HOST],
      rules: {
        "no-restricted-imports": bans(
          [...viewPaths, ...xstate, ...xstateReact],
          [earnestModels, stylesheets, node, mainAndPreload]
        ),
      },
    },
    {
      files: [ENTRY],
      rules: {
        "no-restricted-imports": bans(
          [...viewPaths, ...reactEffects, ...xstate, ...xstateReact],
          [earnestModels, xterm, node, mainAndPreload]
        ),
      },
    },
  ],
  rules: {
    "@stylexjs/enforce-extension": "error",
    "@stylexjs/no-unused": "error",
    "@stylexjs/sort-keys": "error",
    "@stylexjs/valid-shorthands": "error",
    "@stylexjs/valid-styles": "error",
    "no-comments/disallowComments": "error",
    "no-restricted-imports": bans([...net, ...reactState], everywhere),
    "typescript/promise-function-async": "off",
    "unicorn/no-array-method-this-argument": "off",
    "unicorn/filename-case": ["error", { case: "kebabCase" }],
    "wake-styles/keel-class-names": "error",
    "wake-styles/no-raw-style-values": "error",
  },
});
