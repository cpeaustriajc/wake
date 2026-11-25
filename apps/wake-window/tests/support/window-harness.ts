import { Array, Effect, Option } from "effect";
import { SimulatedClock, createActor, fromPromise, waitFor } from "xstate";
import type { Actor } from "xstate";

import { windowMachine } from "#/renderer/window/window-machine.ts";
import type { WindowSnapshot } from "#/renderer/window/window-machine.ts";
import type { Agent, Placement, Position } from "#/shared/agent.ts";
import type { FleetReading, PlaceOutcome } from "#/shared/daemon-answer.ts";

type WindowActor = Actor<typeof windowMachine>;

interface Script {
  readonly outcomes?: readonly PlaceOutcome[];
  readonly readings: readonly FleetReading[];
}

interface Harness {
  readonly actor: WindowActor;
  readonly clock: SimulatedClock;
  readonly sent: Placement[];
}

const POLL = 2000;
const PLACED: PlaceOutcome = { kind: "placed" };
const NO_AGENTS: FleetReading = { agents: [], kind: "agents" };

interface AgentSeed {
  readonly id: number;
  readonly position?: Position | null;
}

const agent = ({ id, position = null }: AgentSeed): Agent => ({
  branch: null,
  folder: `/work/${id}`,
  id,
  location: "local",
  position,
  progress: {
    headline: `Agent ${id}`,
    phase: "working",
    state: "found",
    updated: "2026-11-25T12:00:00Z",
  },
  started_at: 1700 + id,
});

const live = (...agents: readonly Agent[]): FleetReading => ({
  agents,
  kind: "agents",
});

const scripted = <Step>(steps: readonly Step[], fallback: Step) => {
  const taken: Step[] = [];
  return (): Step => {
    const step = Option.getOrElse(
      Option.orElse(Array.get(steps, taken.length), () => Array.last(steps)),
      () => fallback
    );
    taken.push(step);
    return step;
  };
};

const harness = ({ outcomes = [], readings }: Script): Harness => {
  const sent: Placement[] = [];
  const clock = new SimulatedClock();
  const nextReading = scripted(readings, NO_AGENTS);
  const nextOutcome = scripted(outcomes, PLACED);
  const actor = createActor(
    windowMachine.provide({
      actors: {
        placeOnFloor: fromPromise(({ input }) => {
          sent.push(input);
          return Promise.resolve(nextOutcome());
        }),
        readFleet: fromPromise(() =>
          Promise.resolve({ at: clock.now(), reading: nextReading() })
        ),
      },
    }),
    { clock }
  );
  actor.start();
  return { actor, clock, sent };
};

const quiet = (actor: WindowActor): Effect.Effect<WindowSnapshot> =>
  Effect.promise(() =>
    waitFor(
      actor,
      (snapshot) =>
        snapshot.matches({ fleet: { cycle: "waiting" } }) &&
        !snapshot.matches({ placement: { save: "saving" } })
    )
  );

const poll = (harnessed: Harness): Effect.Effect<WindowSnapshot> =>
  Effect.sync(() => {
    harnessed.clock.increment(POLL);
  }).pipe(
    Effect.flatMap(() => Effect.promise(() => Promise.resolve())),
    Effect.flatMap(() => quiet(harnessed.actor))
  );

export { agent, harness, live, poll, quiet };
export type { Harness, WindowActor };
