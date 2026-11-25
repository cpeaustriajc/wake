import { windowSetup } from "#/renderer/window/window-setup-machine.ts";

const navigationRegion = windowSetup.createStateConfig({
  initial: "floor",
  states: {
    floor: {
      on: {
        "workspace.open": {
          actions: {
            params: ({ event }) => event.key,
            type: "openWorkspace",
          },
          guard: { params: ({ event }) => event.key, type: "canOpen" },
          target: "workspace",
        },
      },
    },
    workspace: {
      on: {
        "fleet.read": {
          actions: "closeWorkspace",
          guard: "agentLeft",
          target: "floor",
        },
        "workspace.close": { actions: "closeWorkspace", target: "floor" },
      },
    },
  },
});

export { navigationRegion };
