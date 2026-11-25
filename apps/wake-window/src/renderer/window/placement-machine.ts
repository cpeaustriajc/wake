import { nextPlacement } from "#/renderer/window/window-context-model.ts";
import { windowSetup } from "#/renderer/window/window-setup-machine.ts";

interface Failure {
  readonly event: { readonly error: unknown };
}

const placementRegion = windowSetup.createStateConfig({
  on: { "fleet.read": { actions: "settleFleet" } },
  states: {
    gesture: {
      initial: "idle",
      states: {
        dragging: {
          on: {
            "pointer.move": {
              actions: { params: ({ event }) => event, type: "drag" },
            },
            "pointer.release": { actions: "drop", target: "dropped" },
          },
        },
        dropped: {
          after: { clickGrace: "idle" },
          on: {
            "tile.click": "idle",
            "tile.nudge": {
              actions: { params: ({ event }) => event, type: "nudge" },
              guard: { params: ({ event }) => event.key, type: "canGrip" },
            },
            "tile.press": {
              actions: { params: ({ event }) => event, type: "grip" },
              guard: { params: ({ event }) => event.key, type: "canGrip" },
              target: "pressing",
            },
          },
        },
        idle: {
          on: {
            "tile.click": {
              actions: {
                params: ({ event }) => event.key,
                type: "requestWorkspace",
              },
            },
            "tile.nudge": {
              actions: { params: ({ event }) => event, type: "nudge" },
              guard: { params: ({ event }) => event.key, type: "canGrip" },
            },
            "tile.press": {
              actions: { params: ({ event }) => event, type: "grip" },
              guard: { params: ({ event }) => event.key, type: "canGrip" },
              target: "pressing",
            },
          },
        },
        pressing: {
          on: {
            "pointer.move": {
              actions: { params: ({ event }) => event, type: "drag" },
              guard: { params: ({ event }) => event, type: "pulledFar" },
              target: "dragging",
            },
            "pointer.release": { actions: "release", target: "idle" },
          },
        },
      },
    },
    save: {
      initial: "idle",
      states: {
        couldNotPlace: { always: { guard: "hasSaves", target: "saving" } },
        idle: { always: { guard: "hasSaves", target: "saving" } },
        saving: {
          invoke: {
            input: ({ context }) => nextPlacement(context),
            onDone: [
              {
                actions: {
                  params: ({ event }) => event.output,
                  type: "settleSave",
                },
                guard: {
                  params: ({ event }) => event.output,
                  type: "wasPlaced",
                },
                target: "idle",
              },
              {
                actions: {
                  params: ({ event }) => event.output,
                  type: "settleSave",
                },
                target: "couldNotPlace",
              },
            ],
            onError: {
              actions: {
                params: ({ event }: Failure) => event.error,
                type: "failSave",
              },
              target: "couldNotPlace",
            },
            src: "placeOnFloor",
          },
        },
      },
    },
  },
  type: "parallel",
});

export { placementRegion };
