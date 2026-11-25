import { windowSetup } from "#/renderer/window/window-setup-machine.ts";

interface Failure {
  readonly event: { readonly error: unknown };
}

const fleetRegion = windowSetup.createStateConfig({
  states: {
    cycle: {
      initial: "reading",
      states: {
        reading: {
          invoke: {
            onDone: {
              actions: [
                { params: ({ event }) => event.output, type: "takeReading" },
                "announceRead",
              ],
              target: "waiting",
            },
            onError: {
              actions: [
                {
                  params: ({ event }: Failure) => event.error,
                  type: "failReading",
                },
                "announceRead",
              ],
              target: "waiting",
            },
            src: "readFleet",
          },
        },
        waiting: { after: { poll: "reading" } },
      },
    },
    status: {
      initial: "connecting",
      on: {
        "fleet.read": [
          { guard: "isLive", target: ".live" },
          { guard: "isDown", target: ".daemonDown" },
          { target: ".unreadable" },
        ],
      },
      states: { connecting: {}, daemonDown: {}, live: {}, unreadable: {} },
    },
  },
  type: "parallel",
});

export { fleetRegion };
