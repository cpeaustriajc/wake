import * as stylex from "@stylexjs/stylex";
import { Array } from "effect";
import type { ReactElement } from "react";

import {
  MACHINE_HEIGHT,
  MACHINE_WIDTH,
  firstSlots,
} from "#/renderer/floor/floor-model.ts";
import { color, radius, space } from "#/renderer/keel/keel-tokens.stylex.ts";
import type {
  PlaceFailure,
  Trouble,
} from "#/renderer/window/window-context-model.ts";

const SKELETON_SLOTS = firstSlots(3);

const shimmer = stylex.keyframes({ "50%": { opacity: 0.55 } });

const styles = stylex.create({
  alerts: {
    gap: space.s2,
    display: "flex",
    flexDirection: "column",
    maxWidth: 560,
  },
  card: { maxWidth: 460 },
  notice: {
    inset: 0,
    padding: space.s5,
    placeItems: "center",
    display: "grid",
    position: "absolute",
  },
  place: (x: number, y: number, width: number, height: number) => ({
    height,
    left: x,
    top: y,
    width,
  }),
  skeleton: {
    borderColor: color.hairline,
    borderRadius: radius.panel,
    borderStyle: "solid",
    borderWidth: 1,
    animationDuration: "1.6s",
    animationIterationCount: "infinite",
    animationName: {
      default: shimmer,
      "@media (prefers-reduced-motion: reduce)": "none",
    },
    animationTimingFunction: "ease-in-out",
    backgroundColor: color.surface,
    position: "absolute",
  },
});

const InboxIcon = (): ReactElement => (
  <svg
    aria-hidden="true"
    fill="none"
    height="16"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="1.5"
    viewBox="0 0 16 16"
    width="16"
  >
    <path d="M2.5 9.5l2-5.5h7l2 5.5V12a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1z" />
    <path d="M2.5 9.5h3.25l.75 1.5h3l.75-1.5h3.25" />
  </svg>
);

const AlertIcon = (): ReactElement => (
  <svg
    aria-hidden="true"
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="1.5"
    viewBox="0 0 16 16"
  >
    <path d="M8 2.5 14 13H2z" />
    <path d="M8 6.5v3M8 11.25h.01" />
  </svg>
);

const FloorSkeleton = (): ReactElement => (
  <>
    {Array.map(SKELETON_SLOTS, ({ x, y }) => (
      <span
        {...stylex.props(
          styles.skeleton,
          styles.place(x, y, MACHINE_WIDTH, MACHINE_HEIGHT)
        )}
        aria-hidden="true"
        key={`${x}-${y}`}
      />
    ))}
  </>
);

const DaemonDown = ({ socket }: { readonly socket: string }): ReactElement => (
  <output className="k-alert k-alert--warning">
    <AlertIcon />
    <span className="k-alert__title">The daemon is not running</span>
    <span className="k-alert__body">
      Start it with <code className="k-mono">wake daemon</code>. Agents keep
      running without this window; it only reads {socket}.
    </span>
  </output>
);

const Unreadable = ({ reason }: { readonly reason: string }): ReactElement => (
  <div className="k-alert k-alert--danger" role="alert">
    <AlertIcon />
    <span className="k-alert__title">
      The daemon&apos;s answer cannot be read
    </span>
    <span className="k-alert__body">
      This window and the daemon disagree on the list of agents: {reason}. Build
      both from the same commit.
    </span>
  </div>
);

const NoAgents = (): ReactElement => (
  <div className="k-card">
    <div className="k-empty">
      <span className="k-tile k-tile--control">
        <InboxIcon />
      </span>
      <span className="k-empty__title">No agents yet</span>
      <span className="k-empty__desc">
        Agents started with <code className="k-mono">wake agent start</code> and
        cloud tasks followed with{" "}
        <code className="k-mono">wake cloud track</code> land on this floor.
      </span>
    </div>
  </div>
);

const TroubleAlert = ({
  trouble,
}: {
  readonly trouble: Trouble;
}): ReactElement | null => {
  if (trouble.kind === "daemon_down") {
    return <DaemonDown socket={trouble.socket} />;
  }
  return trouble.kind === "unreadable" ? (
    <Unreadable reason={trouble.reason} />
  ) : null;
};

const FloorNotice = ({
  trouble,
}: {
  readonly trouble: Trouble;
}): ReactElement => (
  <div {...stylex.props(styles.notice)}>
    <div {...stylex.props(styles.card)}>
      {trouble.kind === "none" ? (
        <NoAgents />
      ) : (
        <TroubleAlert trouble={trouble} />
      )}
    </div>
  </div>
);

const PlaceAlert = ({
  failure,
}: {
  readonly failure: PlaceFailure;
}): ReactElement => (
  <output className="k-alert k-alert--warning">
    <AlertIcon />
    <span className="k-alert__title">{failure.key} could not be placed</span>
    <span className="k-alert__body">
      The daemon did not take the new place: {failure.reason}. The machine is
      back where the daemon keeps it.
    </span>
  </output>
);

interface FloorAlertsProps {
  readonly failures: readonly PlaceFailure[];
  readonly stale: boolean;
  readonly trouble: Trouble;
}

const FloorAlerts = ({
  failures,
  stale,
  trouble,
}: FloorAlertsProps): ReactElement | null => {
  const troubled = stale && trouble.kind !== "none";
  if (!troubled && !Array.isReadonlyArrayNonEmpty(failures)) {
    return null;
  }
  return (
    <div {...stylex.props(styles.alerts)}>
      {troubled ? <TroubleAlert trouble={trouble} /> : null}
      {Array.map(failures, (failure) => (
        <PlaceAlert failure={failure} key={failure.key} />
      ))}
    </div>
  );
};

export { FloorAlerts, FloorNotice, FloorSkeleton };
