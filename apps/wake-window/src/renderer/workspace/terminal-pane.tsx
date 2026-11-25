import * as stylex from "@stylexjs/stylex";
import type { ReactElement } from "react";

import type { Tile } from "#/renderer/floor/tile-model.ts";
import {
  color,
  radius,
  shadow,
  space,
} from "#/renderer/keel/keel-tokens.stylex.ts";
import type { TerminalContext } from "#/renderer/window/terminal-context-model.ts";
import type { WindowSnapshot } from "#/renderer/window/window-machine.ts";
import {
  useWindow,
  useWindowSend,
} from "#/renderer/window/window-provider.tsx";
import { TerminalHost } from "#/renderer/workspace/terminal-host.tsx";

type TerminalPhase = "closed" | "exited" | "failed" | "open" | "opening";

const SCREEN_HEIGHT = 420;

const styles = stylex.create({
  pane: { gap: space.s3, display: "flex", flexDirection: "column" },
  screen: {
    padding: space.s2,
    borderColor: color.edgeStrong,
    borderRadius: radius.panel,
    borderStyle: "solid",
    borderWidth: 1,
    overflow: "hidden",
    backgroundColor: color.well,
    boxShadow: shadow.well,
    height: SCREEN_HEIGHT,
  },
  status: { color: color.inkMuted, fontSize: 13 },
});

const phaseIn = (snapshot: WindowSnapshot): TerminalPhase => {
  if (snapshot.matches({ terminal: "open" })) {
    return "open";
  }
  if (snapshot.matches({ terminal: "exited" })) {
    return "exited";
  }
  if (snapshot.matches({ terminal: "failed" })) {
    return "failed";
  }
  return snapshot.matches({ terminal: "opening" }) ? "opening" : "closed";
};

const terminalIn = (snapshot: WindowSnapshot): TerminalContext =>
  snapshot.context.terminal;

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

const ClockIcon = (): ReactElement => (
  <svg
    aria-hidden="true"
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="1.5"
    viewBox="0 0 16 16"
  >
    <circle cx="8" cy="8" r="6" />
    <path d="M8 4.5V8l2.25 1.5" />
  </svg>
);

interface RestartProps {
  readonly label: string;
  readonly onRestart: () => void;
}

const Restart = ({ label, onRestart }: RestartProps): ReactElement => (
  <span className="k-alert__body">
    <button className="k-btn k-btn--sm" onClick={onRestart} type="button">
      {label}
    </button>
  </span>
);

interface NoticeProps {
  readonly onRestart: () => void;
  readonly phase: TerminalPhase;
  readonly terminal: TerminalContext;
}

const Exited = ({ onRestart, terminal }: NoticeProps): ReactElement => (
  <output className="k-alert">
    <ClockIcon />
    <span className="k-alert__title">
      The shell exited with code {terminal.exitCode}
    </span>
    <span className="k-alert__body">
      Its output stays below. A new shell starts in the same folder.
    </span>
    <Restart label="Start a new shell" onRestart={onRestart} />
  </output>
);

const Failed = ({ onRestart, terminal }: NoticeProps): ReactElement => (
  <output className="k-alert k-alert--warning">
    <AlertIcon />
    <span className="k-alert__title">
      {terminal.problem.kind === "daemon_down"
        ? "The daemon is not running"
        : "The daemon refused the terminal"}
    </span>
    <span className="k-alert__body">
      {terminal.problem.kind === "none" ? "" : terminal.problem.reason}. Shells
      end with the daemon, so a new one starts if it was restarted.
    </span>
    <Restart label="Try again" onRestart={onRestart} />
  </output>
);

const Notice = (props: NoticeProps): ReactElement | null => {
  const { phase, terminal } = props;
  if (phase === "exited") {
    return <Exited {...props} />;
  }
  if (phase === "failed") {
    return <Failed {...props} />;
  }
  if (phase !== "open") {
    return (
      <output {...stylex.props(styles.status)}>Starting the shell.</output>
    );
  }
  return terminal.truncated ? (
    <output {...stylex.props(styles.status)}>
      Earlier output is gone: the daemon keeps the last mebibyte of each shell.
    </output>
  ) : null;
};

const LocalTerminal = ({ tile }: { readonly tile: Tile }): ReactElement => {
  const phase = useWindow(phaseIn);
  const terminal = useWindow(terminalIn);
  const send = useWindowSend();
  const handleRestart = (): void => {
    send({ type: "terminal.restart" });
  };
  return (
    <section
      {...stylex.props(styles.pane)}
      aria-label={`Terminal of ${tile.key}`}
    >
      <Notice onRestart={handleRestart} phase={phase} terminal={terminal} />
      <div {...stylex.props(styles.screen)}>
        <TerminalHost tileKey={tile.key} />
      </div>
    </section>
  );
};

const NoLocalShell = (): ReactElement => (
  <div className="k-card">
    <div className="k-empty">
      <span className="k-empty__title">No shell for a cloud task</span>
      <span className="k-empty__desc">
        A cloud task runs on another machine, so there is no folder here to open
        a shell in. Its branch shows what it has done.
      </span>
    </div>
  </div>
);

const TerminalPane = ({ tile }: { readonly tile: Tile }): ReactElement =>
  tile.location === "cloud" ? <NoLocalShell /> : <LocalTerminal tile={tile} />;

export { TerminalPane };
