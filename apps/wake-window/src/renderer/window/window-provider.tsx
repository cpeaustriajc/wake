import { createActorContext } from "@xstate/react";
import { Clock, Effect } from "effect";
import { useCallback, useMemo } from "react";
import type { ReactElement, ReactNode } from "react";

import {
  pickedIn,
  themeIn,
  themeMachine,
} from "#/renderer/theme/theme-machine.ts";
import { DARK_QUERY, THEME_KEY } from "#/renderer/theme/theme-model.ts";
import type { Theme } from "#/renderer/theme/theme-model.ts";
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

interface WindowProviderProps {
  readonly bridge: WakeBridge;
  readonly children: ReactNode;
}

type ThemeEvent = Parameters<
  ReturnType<typeof ThemeActor.useActorRef>["send"]
>[0];

const WindowActor = createActorContext(windowMachine);
const ThemeActor = createActorContext(themeMachine);

const ioOf = (bridge: WakeBridge): WindowIo => ({
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
});

const WindowProvider = ({
  bridge,
  children,
}: WindowProviderProps): ReactElement => {
  const logic = useMemo(() => windowMachineWith(ioOf(bridge)), [bridge]);
  const themeOptions = useMemo(
    () => ({
      input: {
        stored: localStorage.getItem(THEME_KEY),
        systemDark: globalThis.matchMedia(DARK_QUERY).matches,
      },
    }),
    []
  );
  return (
    <ThemeActor.Provider options={themeOptions}>
      <WindowActor.Provider logic={logic}>{children}</WindowActor.Provider>
    </ThemeActor.Provider>
  );
};

const useWindow = <Selected,>(
  selector: (snapshot: WindowSnapshot) => Selected
): Selected => WindowActor.useSelector(selector);

const useWindowSend = (): ((event: WindowEvent) => void) => {
  const actor = WindowActor.useActorRef();
  return useCallback(
    (event: WindowEvent) => {
      actor.send(event);
    },
    [actor]
  );
};

const useTheme = (): Theme => ThemeActor.useSelector(themeIn);

const usePickedTheme = (): Theme | null => ThemeActor.useSelector(pickedIn);

const useThemeSend = (): ((event: ThemeEvent) => void) => {
  const actor = ThemeActor.useActorRef();
  return useCallback(
    (event: ThemeEvent) => {
      actor.send(event);
    },
    [actor]
  );
};

export {
  WindowProvider,
  usePickedTheme,
  useTheme,
  useThemeSend,
  useWindow,
  useWindowSend,
};
