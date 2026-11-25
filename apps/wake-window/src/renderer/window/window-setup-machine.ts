import { assign, fromPromise, raise, setup } from "xstate";

import {
  INITIAL_CONTEXT,
  NO_KEY,
  agentLeft,
  canGrip,
  canOpen,
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

const CLICK_GRACE_MILLISECONDS = 100;
const POLL_MILLISECONDS = 2000;
const anyEvent = (): WindowEvent => ({ type: "workspace.close" });

const windowSetup = setup({
  actions: {
    announceRead: raise({ type: "fleet.read" }),
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
    grip: assign(({ context }, press: Press) => gripped({ context, press })),
    nudge: assign(({ context }, nudge: Nudge) => nudged({ context, nudge })),
    openWorkspace: assign((_args, key: string) => ({ openedKey: key })),
    release: assign(() => released()),
    requestWorkspace: raise((_args, key: string) => ({
      key,
      type: "workspace.open" as const,
    })),
    settleFleet: assign(({ context }) => fleetSettled(context)),
    settleSave: assign(({ context }, outcome: PlaceOutcome) =>
      saveSettled({ context, outcome })
    ),
    takeReading: assign((_args, snapshot: FleetSnapshot) =>
      readingTaken(snapshot)
    ),
  },
  actors: {
    placeOnFloor: fromPromise<PlaceOutcome, Placement>(() =>
      Promise.reject(new Error("placeOnFloor is not provided"))
    ),
    readFleet: fromPromise<FleetSnapshot>(() =>
      Promise.reject(new Error("readFleet is not provided"))
    ),
  },
  delays: {
    clickGrace: CLICK_GRACE_MILLISECONDS,
    poll: POLL_MILLISECONDS,
  },
  guards: {
    agentLeft: ({ context }) => agentLeft(context),
    canGrip: ({ context }, key: string) => canGrip({ context, key }),
    canOpen: ({ context }, key: string) => canOpen({ context, key }),
    hasSaves: ({ context }) => hasSaves(context),
    isDown: ({ context }) => context.trouble.kind === "daemon_down",
    isLive: ({ context }) => context.trouble.kind === "none",
    pulledFar: ({ context }, pointer: Pointer) =>
      pulledFar({ context, pointer }),
    wasPlaced: (_args, outcome: PlaceOutcome) => wasPlaced(outcome),
  },
  types: { context: INITIAL_CONTEXT, events: anyEvent() },
});

export { windowSetup };
