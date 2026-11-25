import {
  inputOf,
  openingOf,
  readingOf,
  resizeOf,
} from "#/renderer/window/terminal-context-model.ts";
import { terminalOwner } from "#/renderer/window/window-context-model.ts";
import { windowSetup } from "#/renderer/window/window-setup-machine.ts";

interface Failure {
  readonly event: { readonly error: unknown };
}

const terminalRegion = windowSetup.createStateConfig({
  initial: "closed",
  on: {
    "fleet.read": {
      actions: "hideTerminal",
      guard: "terminalAgentLeft",
      target: ".closed",
    },
    "terminal.fit": {
      actions: { params: ({ event }) => event, type: "fitTerminal" },
    },
    "terminal.hide": { actions: "hideTerminal", target: ".closed" },
    "terminal.show": {
      actions: { params: ({ event }) => event, type: "showTerminal" },
      guard: { params: ({ event }) => event.key, type: "canShowTerminal" },
      target: ".opening",
    },
    "workspace.close": { actions: "hideTerminal", target: ".closed" },
  },
  states: {
    closed: {},
    exited: {
      id: "terminalExited",
      on: {
        "terminal.restart": { actions: "restartTerminal", target: "opening" },
      },
    },
    failed: {
      id: "terminalFailed",
      on: {
        "terminal.restart": { actions: "restartTerminal", target: "opening" },
      },
    },
    open: {
      on: {
        "terminal.type": {
          actions: {
            params: ({ event }) => event.data,
            type: "typeIntoTerminal",
          },
        },
      },
      states: {
        read: {
          initial: "waiting",
          states: {
            waiting: {
              invoke: {
                input: ({ context }) => readingOf(context.terminal),
                onDone: [
                  {
                    actions: [
                      {
                        params: ({ event }) => event.output,
                        type: "feedOutput",
                      },
                      {
                        params: ({ event }) => event.output,
                        type: "takeOutput",
                      },
                    ],
                    guard: {
                      params: ({ event }) => event.output,
                      type: "sawExit",
                    },
                    target: "#terminalExited",
                  },
                  {
                    actions: [
                      {
                        params: ({ event }) => event.output,
                        type: "feedOutput",
                      },
                      {
                        params: ({ event }) => event.output,
                        type: "takeOutput",
                      },
                    ],
                    guard: {
                      params: ({ event }) => event.output,
                      type: "gaveOutput",
                    },
                    reenter: true,
                    target: "waiting",
                  },
                  {
                    actions: {
                      params: ({ event }) => event.output,
                      type: "takeOutput",
                    },
                    target: "#terminalFailed",
                  },
                ],
                onError: {
                  actions: {
                    params: ({ event }: Failure) => event.error,
                    type: "failTerminal",
                  },
                  target: "#terminalFailed",
                },
                src: "readTerminal",
              },
            },
          },
        },
        size: {
          initial: "settled",
          states: {
            debouncing: {
              after: {
                resizeDebounce: { actions: "claimSize", target: "sending" },
              },
              on: {
                "terminal.fit": {
                  actions: {
                    params: ({ event }) => event,
                    type: "fitTerminal",
                  },
                  reenter: true,
                  target: "debouncing",
                },
              },
            },
            sending: {
              invoke: {
                input: ({ context }) => resizeOf(context.terminal),
                onDone: { actions: "settleSize", target: "settled" },
                onError: { actions: "settleSize", target: "settled" },
                src: "resizeTerminal",
              },
            },
            settled: {
              always: { guard: "sizeDiffers", target: "debouncing" },
            },
          },
        },
        write: {
          initial: "idle",
          states: {
            idle: {
              always: {
                actions: "claimTyped",
                guard: "hasTyped",
                target: "sending",
              },
            },
            sending: {
              invoke: {
                input: ({ context }) => inputOf(context.terminal),
                onDone: { actions: "settleTyped", target: "idle" },
                onError: { actions: "settleTyped", target: "idle" },
                src: "writeTerminal",
              },
            },
          },
        },
      },
      type: "parallel",
    },
    opening: {
      invoke: {
        input: ({ context }) =>
          openingOf({
            owner: terminalOwner(context),
            terminal: context.terminal,
          }),
        onDone: [
          {
            actions: [
              "feedReset",
              { params: ({ event }) => event.output, type: "takeOpened" },
            ],
            guard: {
              params: ({ event }) => event.output,
              type: "startsAfresh",
            },
            target: "open",
          },
          {
            actions: {
              params: ({ event }) => event.output,
              type: "takeOpened",
            },
            guard: { params: ({ event }) => event.output, type: "wasOpened" },
            target: "open",
          },
          {
            actions: {
              params: ({ event }) => event.output,
              type: "takeOpened",
            },
            target: "failed",
          },
        ],
        onError: {
          actions: {
            params: ({ event }: Failure) => event.error,
            type: "failTerminal",
          },
          target: "failed",
        },
        src: "openTerminal",
      },
    },
  },
});

export { terminalRegion };
