import { fromPromise } from "xstate";
import type { SnapshotFrom } from "xstate";

import { fleetRegion } from "#/renderer/window/fleet-machine.ts";
import { navigationRegion } from "#/renderer/window/navigation-machine.ts";
import { placementRegion } from "#/renderer/window/placement-machine.ts";
import { terminalRegion } from "#/renderer/window/terminal-machine.ts";
import { INITIAL_CONTEXT } from "#/renderer/window/window-context-model.ts";
import type { FleetSnapshot } from "#/renderer/window/window-context-model.ts";
import { windowSetup } from "#/renderer/window/window-setup-machine.ts";
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

interface WindowIo {
  readonly openTerminal: (opening: Opening) => Promise<OpenOutcome>;
  readonly placeOnFloor: (placement: Placement) => Promise<PlaceOutcome>;
  readonly readFleet: () => Promise<FleetSnapshot>;
  readonly readTerminal: (reading: Reading) => Promise<ReadOutcome>;
  readonly resizeTerminal: (size: Resize) => Promise<SentOutcome>;
  readonly writeTerminal: (input: Input) => Promise<SentOutcome>;
}

const windowMachine = windowSetup.createMachine({
  context: INITIAL_CONTEXT,
  id: "window",
  states: {
    fleet: fleetRegion,
    navigation: navigationRegion,
    placement: placementRegion,
    terminal: terminalRegion,
  },
  type: "parallel",
});

const windowMachineWith = (io: WindowIo): typeof windowMachine =>
  windowMachine.provide({
    actors: {
      openTerminal: fromPromise(({ input }) => io.openTerminal(input)),
      placeOnFloor: fromPromise(({ input }) => io.placeOnFloor(input)),
      readFleet: fromPromise(() => io.readFleet()),
      readTerminal: fromPromise(({ input }) => io.readTerminal(input)),
      resizeTerminal: fromPromise(({ input }) => io.resizeTerminal(input)),
      writeTerminal: fromPromise(({ input }) => io.writeTerminal(input)),
    },
  });

type WindowSnapshot = SnapshotFrom<typeof windowMachine>;

export { windowMachine, windowMachineWith };
export type { WindowIo, WindowSnapshot };
