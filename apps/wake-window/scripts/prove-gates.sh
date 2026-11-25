#!/bin/sh
set -u
cd "$(dirname "$0")/.."

failures=0

probe() {
  name=$1
  marker=$2
  file=$3
  content=$4
  printf '%s\n' "$content" > "$file"
  output=$(npx effect-tsgo oxlint --type-aware "$file" 2>&1)
  status=$?
  rm -f "$file"
  if [ "$status" -ne 0 ] && printf '%s\n' "$output" | grep -qF -- "$marker"; then
    echo "FAILS AS IT MUST $name"
  else
    echo "NOT CAUGHT       $name"
    failures=$((failures + 1))
  fi
  printf '%s\n' "$output" | grep -F -- "$marker" | awk '!seen[$0]++' | cut -c1-180 | sed 's/^/    /'
}

probe "a view imports electron and node" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { ipcRenderer } from "electron";
import { readFileSync } from "node:fs";
export { ipcRenderer, readFileSync };'

probe "a view holds state or runs an effect" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { useEffect, useReducer, useState, useSyncExternalStore } from "react";
export { useEffect, useReducer, useState, useSyncExternalStore };'

probe "a view memoizes by hand" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { memo, useCallback, useMemo } from "react";
export { memo, useCallback, useMemo };'

probe "a view imports XState" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { createActorContext } from "@xstate/react";
import { createActor } from "xstate";
export { createActor, createActorContext };'

probe "a view other than the terminal host draws with xterm" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
export { FitAddon, Terminal };'

probe "the main process imports xterm" "no-restricted-imports" src/main/probe.ts \
  'import { Terminal } from "@xterm/xterm";
export { Terminal };'

probe "a view imports the main process" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { askDaemon } from "#/main/daemon-client.ts";
export { askDaemon };'

probe "a view imports a stylesheet" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import "#/renderer/keel/tokens.css";
export const probe = 1;'

probe "a machine imports React, UI SDK or a view" "no-restricted-imports" src/renderer/window/probe-machine.ts \
  'import { Tabs } from "@ui-sdk/react/tabs";
import { createElement } from "react";
import { App } from "#/renderer/app.tsx";
export { App, Tabs, createElement };'

probe "a model imports XState" "no-restricted-imports" src/renderer/floor/probe-model.ts \
  'import { createActor } from "xstate";
export { createActor };'

probe "the main process imports React or XState" "no-restricted-imports" src/main/probe.ts \
  'import { createElement } from "react";
import { createActor } from "xstate";
export { createActor, createElement };'

probe "the preload imports a library or opens a socket" "no-restricted-imports" src/preload/probe.ts \
  'import { Effect } from "effect";
import { createConnection } from "node:net";
export { Effect, createConnection };'

probe "src/shared imports the renderer" "no-restricted-imports" src/shared/probe.ts \
  'import { tilesOf } from "#/renderer/floor/tile-model.ts";
export { tilesOf };'

probe "a socket outside src/main" "no-restricted-imports" tests/probe.ts \
  'import { createConnection } from "node:net";
export { createConnection };'

probe "something imports an earnest model" "no-restricted-imports" src/renderer/probe-view.tsx \
  'import { Fleet } from "#/renderer/window/fleet.model.ts";
export { Fleet };'

probe "a comment" "no-comments" src/shared/probe.ts \
  '// a note
export const probe = 1;'

probe "a raw colour, shadow, radius, font or !important in a style" "no-raw-style-values" src/renderer/probe-view.tsx \
  'import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({
  probe: {
    borderRadius: "4px",
    boxShadow: "0 1px 2px black",
    color: "#ff0000",
    fontFamily: "Arial",
    width: "10px !important",
  },
});
export const probe = stylex.props(styles.probe);'

probe "a hand-written class or style prop" "keel-class-names" src/renderer/probe-view.tsx \
  'export const Probe = () => (
  <div className="wk-box" style={{ left: 1 }}>
    probe
  </div>
);'

probe "an invalid StyleX property" "@stylexjs" src/renderer/probe-view.tsx \
  'import * as stylex from "@stylexjs/stylex";
const styles = stylex.create({ probe: { border: "1px solid", colour: "inherit" } });
export const probe = stylex.props(styles.probe);'

echo "gates not caught: $failures"
[ "$failures" -eq 0 ]
