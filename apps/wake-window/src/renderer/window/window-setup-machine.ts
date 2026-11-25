import { assign, emit, fromPromise, raise, setup } from "xstate";

import {
  INITIAL_TERMINAL,
  bytesOf,
  gaveOutput,
  hasTyped,
  sawExit,
  sizeClaimed,
  sizeDiffers,
  sizeSettled,
  startsAfresh,
  terminalFailed,
  terminalFitted,
  terminalOpened,
  terminalRead,
  terminalRestarted,
  terminalShown,
  terminalTyped,
  typedClaimed,
  typedSettled,
  wasOpened,
} from "#/renderer/window/terminal-context-model.ts";
import type {
  TerminalCells,
  TerminalFeed,
  TerminalShow,
} from "#/renderer/window/terminal-context-model.ts";
import {
  INITIAL_CONTEXT,
  NO_KEY,
  agentLeft,
  canGrip,
  canOpen,
  canShowTerminal,
  dragged,
  dropped,
  fleetSettled,
  gripped,
  hasSaves,
  nudged,
  pulledFar,
  readingFailed,
  readingTaken,
  reasonOf,
  released,
  saveSettled,
  terminalAgentLeft,
  wasPlaced,
} from "#/renderer/window/window-context-model.ts";
import type {
  FleetSnapshot,
  Nudge,
  Pointer,
  Press,
  WindowEvent,
} from "#/renderer/window/window-context-model.ts";
import type { Placement } from "#/shared/agent.ts";
import type { PlaceOutcome } from "#/shared/daemon-answer.ts";
import type {
  Input,
  OpenOutcome,
  Opening,
  ReadOutcome,
  Reading,
  Resize,
  SentOutcome,
} from "#/shared/terminal.ts";

const CLICK_GRACE_MILLISECONDS = 100;
const POLL_MILLISECONDS = 2000;
const RESIZE_DEBOUNCE_MILLISECONDS = 150;
const anyEvent = (): WindowEvent => ({ type: "workspace.close" });
const anyFeed = (): TerminalFeed => ({ type: "terminal.reset" });

const windowSetup = setup({
  actions: {
    announceRead: raise({ type: "fleet.read" }),
    claimSize: assign(({ context }) => ({
      terminal: sizeClaimed(context.terminal),
    })),
    claimTyped: assign(({ context }) => ({
      terminal: typedClaimed(context.terminal),
    })),
    closeWorkspace: assign({ openedKey: NO_KEY }),
    drag: assign(({ context }, pointer: Pointer) =>
      dragged({ context, pointer })
    ),
    drop: assign(({ context }) => dropped(context)),
    failReading: assign((_args, error: unknown) => readingFailed(error)),
    failSave: assign(({ context }, error: unknown) =>
      saveSettled({
        context,
        outcome: { kind: "refused", reason: reasonOf(error) },
      })
    ),
    failTerminal: assign(({ context }, error: unknown) => ({
      terminal: terminalFailed({
        reason: reasonOf(error),
        terminal: context.terminal,
      }),
    })),
    feedOutput: emit((_args, outcome: ReadOutcome) => ({
      bytes: bytesOf(outcome),
      type: "terminal.output" as const,
    })),
    feedReset: emit({ type: "terminal.reset" }),
    fitTerminal: assign(({ context }, cells: TerminalCells) => ({
      terminal: terminalFitted({ cells, terminal: context.terminal }),
    })),
    grip: assign(({ context }, press: Press) => gripped({ context, press })),
    hideTerminal: assign({ terminal: INITIAL_TERMINAL }),
    nudge: assign(({ context }, nudge: Nudge) => nudged({ context, nudge })),
    openWorkspace: assign((_args, key: string) => ({ openedKey: key })),
    release: assign(() => released()),
    requestWorkspace: raise((_args, key: string) => ({
      key,
      type: "workspace.open" as const,
    })),
    restartTerminal: assign(({ context }) => ({
      terminal: terminalRestarted(context.terminal),
    })),
    settleFleet: assign(({ context }) => fleetSettled(context)),
    settleSave: assign(({ context }, outcome: PlaceOutcome) =>
      saveSettled({ context, outcome })
    ),
    settleSize: assign(({ context }) => ({
      terminal: sizeSettled(context.terminal),
    })),
    settleTyped: assign(({ context }) => ({
      terminal: typedSettled(context.terminal),
    })),
    showTerminal: assign((_args, show: TerminalShow) => ({
      terminal: terminalShown(show),
    })),
    takeOpened: assign(({ context }, outcome: OpenOutcome) => ({
      terminal: terminalOpened({ outcome, terminal: context.terminal }),
    })),
    takeOutput: assign(({ context }, outcome: ReadOutcome) => ({
      terminal: terminalRead({ outcome, terminal: context.terminal }),
    })),
    takeReading: assign((_args, snapshot: FleetSnapshot) =>
      readingTaken(snapshot)
    ),
    typeIntoTerminal: assign(({ context }, data: string) => ({
      terminal: terminalTyped({ data, terminal: context.terminal }),
    })),
  },
  actors: {
    openTerminal: fromPromise<OpenOutcome, Opening>(() =>
      Promise.reject(new Error("openTerminal is not provided"))
    ),
    placeOnFloor: fromPromise<PlaceOutcome, Placement>(() =>
      Promise.reject(new Error("placeOnFloor is not provided"))
    ),
    readFleet: fromPromise<FleetSnapshot>(() =>
      Promise.reject(new Error("readFleet is not provided"))
    ),
    readTerminal: fromPromise<ReadOutcome, Reading>(() =>
      Promise.reject(new Error("readTerminal is not provided"))
    ),
    resizeTerminal: fromPromise<SentOutcome, Resize>(() =>
      Promise.reject(new Error("resizeTerminal is not provided"))
    ),
    writeTerminal: fromPromise<SentOutcome, Input>(() =>
      Promise.reject(new Error("writeTerminal is not provided"))
    ),
  },
  delays: {
    clickGrace: CLICK_GRACE_MILLISECONDS,
    poll: POLL_MILLISECONDS,
    resizeDebounce: RESIZE_DEBOUNCE_MILLISECONDS,
  },
  guards: {
    agentLeft: ({ context }) => agentLeft(context),
    canGrip: ({ context }, key: string) => canGrip({ context, key }),
    canOpen: ({ context }, key: string) => canOpen({ context, key }),
    canShowTerminal: ({ context }, key: string) =>
      canShowTerminal({ context, key }),
    gaveOutput: (_args, outcome: ReadOutcome) => gaveOutput(outcome),
    hasSaves: ({ context }) => hasSaves(context),
    hasTyped: ({ context }) => hasTyped(context.terminal),
    isDown: ({ context }) => context.trouble.kind === "daemon_down",
    isLive: ({ context }) => context.trouble.kind === "none",
    pulledFar: ({ context }, pointer: Pointer) =>
      pulledFar({ context, pointer }),
    sawExit: (_args, outcome: ReadOutcome) => sawExit(outcome),
    sizeDiffers: ({ context }) => sizeDiffers(context.terminal),
    startsAfresh: ({ context }, outcome: OpenOutcome) =>
      startsAfresh({ outcome, terminal: context.terminal }),
    terminalAgentLeft: ({ context }) => terminalAgentLeft(context),
    wasOpened: (_args, outcome: OpenOutcome) => wasOpened(outcome),
    wasPlaced: (_args, outcome: PlaceOutcome) => wasPlaced(outcome),
  },
  types: { context: INITIAL_CONTEXT, emitted: anyFeed(), events: anyEvent() },
});

export { windowSetup };
