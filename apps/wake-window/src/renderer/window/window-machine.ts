import { fromPromise } from "xstate";
import type { SnapshotFrom } from "xstate";

import { fleetRegion } from "#/renderer/window/fleet-machine.ts";
import { navigationRegion } from "#/renderer/window/navigation-machine.ts";
import { placementRegion } from "#/renderer/window/placement-machine.ts";
import { INITIAL_CONTEXT } from "#/renderer/window/window-context-model.ts";
import type { FleetSnapshot } from "#/renderer/window/window-context-model.ts";
import { windowSetup } from "#/renderer/window/window-setup-machine.ts";
import type { Placement } from "#/shared/agent.ts";
import type { PlaceOutcome } from "#/shared/daemon-answer.ts";

interface WindowIo {
  readonly placeOnFloor: (placement: Placement) => Promise<PlaceOutcome>;
  readonly readFleet: () => Promise<FleetSnapshot>;
}

const windowMachine = windowSetup.createMachine({
  context: INITIAL_CONTEXT,
  id: "window",
  states: {
    fleet: fleetRegion,
    navigation: navigationRegion,
    placement: placementRegion,
  },
  type: "parallel",
});

const windowMachineWith = (io: WindowIo): typeof windowMachine =>
  windowMachine.provide({
    actors: {
      placeOnFloor: fromPromise(({ input }) => io.placeOnFloor(input)),
      readFleet: fromPromise(() => io.readFleet()),
    },
  });

type WindowSnapshot = SnapshotFrom<typeof windowMachine>;

export { windowMachine, windowMachineWith };
export type { WindowIo, WindowSnapshot };
