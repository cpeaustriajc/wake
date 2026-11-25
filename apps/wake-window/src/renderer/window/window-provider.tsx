import { createActorContext } from "@xstate/react";
import { Clock, Effect } from "effect";
import type { ReactElement, ReactNode } from "react";

import {
  pickedIn,
  themeIn,
  themeMachine,
} from "#/renderer/theme/theme-machine.ts";
import { DARK_QUERY, THEME_KEY } from "#/renderer/theme/theme-model.ts";
import type { Theme } from "#/renderer/theme/theme-model.ts";
import type { TerminalFeed } from "#/renderer/window/terminal-context-model.ts";
import type { WindowEvent } from "#/renderer/window/window-context-model.ts";
import {
  windowMachine,
  windowMachineWith,
} from "#/renderer/window/window-machine.ts";
import type {
  WindowIo,
  WindowSnapshot,
} from "#/renderer/window/window-machine.ts";
import type { WakeBridge } from "#/shared/bridge.ts";
import {
  decodeFleetReading,
  decodePlaceOutcome,
} from "#/shared/daemon-answer.ts";
import {
  decodeOpenOutcome,
  decodeReadOutcome,
  decodeSentOutcome,
} from "#/shared/terminal.ts";

type FeedListener = (feed: TerminalFeed) => void;
type FeedSubscribe = (listener: FeedListener) => () => void;

interface ThemeOptions {
  readonly input: {
    readonly stored: string | null;
    readonly systemDark: boolean;
  };
}

interface WindowProviderProps {
  readonly children: ReactNode;
  readonly logic: typeof windowMachine;
  readonly themeOptions: ThemeOptions;
}

type ThemeEvent = Parameters<
  ReturnType<typeof ThemeActor.useActorRef>["send"]
>[0];

const WindowActor = createActorContext(windowMachine);
const ThemeActor = createActorContext(themeMachine);

const ioOf = (bridge: WakeBridge): WindowIo => ({
  openTerminal: (opening) =>
    Effect.runPromise(
      Effect.promise(() => bridge.openTerminal(opening)).pipe(
        Effect.flatMap(decodeOpenOutcome)
      )
    ),
  placeOnFloor: (placement) =>
    Effect.runPromise(
      Effect.promise(() => bridge.placeOnFloor(placement)).pipe(
        Effect.flatMap(decodePlaceOutcome)
      )
    ),
  readFleet: () =>
    Effect.runPromise(
      Effect.all({
        at: Clock.currentTimeMillis,
        reading: Effect.promise(() => bridge.readFleet()).pipe(
          Effect.flatMap(decodeFleetReading)
        ),
      })
    ),
  readTerminal: (reading) =>
    Effect.runPromise(
      Effect.promise(() => bridge.readTerminal(reading)).pipe(
        Effect.flatMap(decodeReadOutcome)
      )
    ),
  resizeTerminal: (size) =>
    Effect.runPromise(
      Effect.promise(() => bridge.resizeTerminal(size)).pipe(
        Effect.flatMap(decodeSentOutcome)
      )
    ),
  writeTerminal: (input) =>
    Effect.runPromise(
      Effect.promise(() => bridge.writeTerminal(input)).pipe(
        Effect.flatMap(decodeSentOutcome)
      )
    ),
});

const windowLogicFor = (bridge: WakeBridge): typeof windowMachine =>
  windowMachineWith(ioOf(bridge));

const themeOptionsNow = (): ThemeOptions => ({
  input: {
    stored: localStorage.getItem(THEME_KEY),
    systemDark: globalThis.matchMedia(DARK_QUERY).matches,
  },
});

const WindowProvider = ({
  children,
  logic,
  themeOptions,
}: WindowProviderProps): ReactElement => (
  <ThemeActor.Provider options={themeOptions}>
    <WindowActor.Provider logic={logic}>{children}</WindowActor.Provider>
  </ThemeActor.Provider>
);

const useWindow = <Selected,>(
  selector: (snapshot: WindowSnapshot) => Selected
): Selected => WindowActor.useSelector(selector);

const useWindowSend = (): ((event: WindowEvent) => void) => {
  const actor = WindowActor.useActorRef();
  return (event: WindowEvent) => {
    actor.send(event);
  };
};

const useTerminalFeed = (): FeedSubscribe => {
  const actor = WindowActor.useActorRef();
  return (listener) => {
    const output = actor.on("terminal.output", listener);
    const reset = actor.on("terminal.reset", listener);
    return () => {
      output.unsubscribe();
      reset.unsubscribe();
    };
  };
};

const useTheme = (): Theme => ThemeActor.useSelector(themeIn);

const usePickedTheme = (): Theme | null => ThemeActor.useSelector(pickedIn);

const useThemeSend = (): ((event: ThemeEvent) => void) => {
  const actor = ThemeActor.useActorRef();
  return (event: ThemeEvent) => {
    actor.send(event);
  };
};

export {
  WindowProvider,
  themeOptionsNow,
  usePickedTheme,
  useTerminalFeed,
  useTheme,
  useThemeSend,
  useWindow,
  useWindowSend,
  windowLogicFor,
};
