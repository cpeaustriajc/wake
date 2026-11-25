import type { FleetReading } from "#/shared/fleet-reading.ts";
import { For, Match, Switch } from "solid-js";
import type { JSX } from "@solidjs/web";
import { slotAt } from "#/renderer/floor/floor-model.ts";

interface FloorNoticeProperties {
  readonly reading: FleetReading;
}

const SKELETON_SLOTS = [0, 1, 2].map((index) => slotAt(index));

const InboxIcon = (): JSX.Element => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M2.5 9.5l2-5.5h7l2 5.5V12a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1z" />
    <path d="M2.5 9.5h3.25l.75 1.5h3l.75-1.5h3.25" />
  </svg>
);

const AlertIcon = (): JSX.Element => (
  <svg
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M8 2.5 14 13H2z" />
    <path d="M8 6.5v3M8 11.25h.01" />
  </svg>
);

const FloorSkeleton = (): JSX.Element => (
  <For each={SKELETON_SLOTS}>
    {(slot) => (
      <span
        class="k-skeleton wk-tile-skeleton"
        style={{ left: `${slot.x}px`, top: `${slot.y}px` }}
        aria-hidden="true"
      />
    )}
  </For>
);

const FloorNotice = (properties: FloorNoticeProperties): JSX.Element => (
  <div class="wk-notice">
    <Switch>
      <Match when={properties.reading.kind === "daemon_down" && properties.reading}>
        {(down) => (
          <div class="k-alert k-alert--warning wk-notice__card" role="status">
            <AlertIcon />
            <span class="k-alert__title">The daemon is not running</span>
            <span class="k-alert__body">
              Start it with <code class="k-mono">wake daemon</code>. Agents keep running without
              this window; it only reads {down().socket}.
            </span>
          </div>
        )}
      </Match>
      <Match when={properties.reading.kind === "unreadable" && properties.reading}>
        {(unreadable) => (
          <div class="k-alert k-alert--danger wk-notice__card" role="alert">
            <AlertIcon />
            <span class="k-alert__title">The daemon's answer cannot be read</span>
            <span class="k-alert__body">
              This window and the daemon disagree on the list of agents: {unreadable().reason}.
              Build both from the same commit.
            </span>
          </div>
        )}
      </Match>
      <Match when={properties.reading.kind === "agents"}>
        <div class="k-card wk-notice__card">
          <div class="k-empty">
            <span class="k-tile k-tile--control">
              <InboxIcon />
            </span>
            <span class="k-empty__title">No agents yet</span>
            <span class="k-empty__desc">
              Agents started with <code class="k-mono">wake agent start</code> and cloud tasks
              followed with <code class="k-mono">wake cloud track</code> land on this floor.
            </span>
          </div>
        </div>
      </Match>
    </Switch>
  </div>
);

export { FloorNotice, FloorSkeleton };
