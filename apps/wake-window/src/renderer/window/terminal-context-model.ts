import { Array } from "effect";

import type {
  Input,
  OpenOutcome,
  Opening,
  ReadOutcome,
  Reading,
  Resize,
  TerminalTrouble,
} from "#/shared/terminal.ts";

interface TerminalCells {
  readonly cols: number;
  readonly rows: number;
}

type TerminalProblem = TerminalTrouble | { readonly kind: "none" };

interface TerminalContext {
  readonly exitCode: number | null;
  readonly id: number;
  readonly key: string;
  readonly next: number;
  readonly problem: TerminalProblem;
  readonly sending: string;
  readonly sendingSize: TerminalCells;
  readonly sentSize: TerminalCells;
  readonly size: TerminalCells;
  readonly truncated: boolean;
  readonly typed: readonly string[];
}

interface TerminalShow extends TerminalCells {
  readonly key: string;
}

interface TerminalOwner {
  readonly id: number;
  readonly location: "cloud" | "local";
}

type TerminalFeed =
  | { readonly bytes: Uint8Array; readonly type: "terminal.output" }
  | { readonly type: "terminal.reset" };

const NO_TERMINAL = 0;
const NO_TERMINAL_KEY = "";
const NO_CELLS: TerminalCells = { cols: 0, rows: 0 };
const NO_PROBLEM: TerminalProblem = { kind: "none" };
const READ_WAIT_MILLISECONDS = 10_000;

const INITIAL_TERMINAL: TerminalContext = {
  exitCode: null,
  id: NO_TERMINAL,
  key: NO_TERMINAL_KEY,
  next: 0,
  problem: NO_PROBLEM,
  sending: "",
  sendingSize: NO_CELLS,
  sentSize: NO_CELLS,
  size: NO_CELLS,
  truncated: false,
  typed: [],
};

const sameCells = (left: TerminalCells, right: TerminalCells): boolean =>
  left.cols === right.cols && left.rows === right.rows;

const terminalShown = ({ cols, key, rows }: TerminalShow): TerminalContext => ({
  ...INITIAL_TERMINAL,
  key,
  sendingSize: { cols, rows },
  size: { cols, rows },
});

interface WithCells {
  readonly cells: TerminalCells;
  readonly terminal: TerminalContext;
}

const terminalFitted = ({ cells, terminal }: WithCells): TerminalContext => ({
  ...terminal,
  size: { cols: cells.cols, rows: cells.rows },
});

const terminalRestarted = (terminal: TerminalContext): TerminalContext => ({
  ...terminal,
  exitCode: null,
  problem: NO_PROBLEM,
  sending: "",
  sendingSize: terminal.size,
  typed: [],
});

interface WithOwner {
  readonly owner: TerminalOwner;
  readonly terminal: TerminalContext;
}

const openingOf = ({ owner, terminal }: WithOwner): Opening => ({
  cols: terminal.sendingSize.cols,
  id: owner.id,
  location: owner.location,
  rows: terminal.sendingSize.rows,
});

interface WithOpened {
  readonly outcome: OpenOutcome;
  readonly terminal: TerminalContext;
}

const startsAfresh = ({ outcome, terminal }: WithOpened): boolean =>
  outcome.kind === "opened" && outcome.terminal !== terminal.id;

const terminalOpened = ({ outcome, terminal }: WithOpened): TerminalContext => {
  if (outcome.kind !== "opened") {
    return { ...terminal, problem: outcome };
  }
  const afresh = startsAfresh({ outcome, terminal });
  return {
    ...terminal,
    exitCode: null,
    id: outcome.terminal,
    next: afresh ? 0 : terminal.next,
    problem: NO_PROBLEM,
    sentSize: terminal.sendingSize,
    truncated: afresh ? false : terminal.truncated,
  };
};

const readingOf = (terminal: TerminalContext): Reading => ({
  after: terminal.next,
  terminal: terminal.id,
  wait_ms: READ_WAIT_MILLISECONDS,
});

interface WithRead {
  readonly outcome: ReadOutcome;
  readonly terminal: TerminalContext;
}

const terminalRead = ({ outcome, terminal }: WithRead): TerminalContext => {
  if (outcome.kind !== "output") {
    return { ...terminal, problem: outcome, sending: "", typed: [] };
  }
  const ended = outcome.exited !== null;
  return {
    ...terminal,
    exitCode: outcome.exited,
    next: outcome.next,
    sending: ended ? "" : terminal.sending,
    truncated: terminal.truncated || outcome.truncated,
    typed: ended ? [] : terminal.typed,
  };
};

interface WithReason {
  readonly reason: string;
  readonly terminal: TerminalContext;
}

const terminalFailed = ({ reason, terminal }: WithReason): TerminalContext => ({
  ...terminal,
  problem: { kind: "refused", reason },
  sending: "",
  typed: [],
});

interface WithTyped {
  readonly data: string;
  readonly terminal: TerminalContext;
}

const terminalTyped = ({ data, terminal }: WithTyped): TerminalContext => ({
  ...terminal,
  typed: [...terminal.typed, data],
});

const hasTyped = (terminal: TerminalContext): boolean =>
  Array.isReadonlyArrayNonEmpty(terminal.typed);

const typedClaimed = (terminal: TerminalContext): TerminalContext => ({
  ...terminal,
  sending: Array.join(terminal.typed, ""),
  typed: [],
});

const inputOf = (terminal: TerminalContext): Input => ({
  data: terminal.sending,
  terminal: terminal.id,
});

const typedSettled = (terminal: TerminalContext): TerminalContext => ({
  ...terminal,
  sending: "",
});

const sizeDiffers = (terminal: TerminalContext): boolean =>
  !sameCells(terminal.size, terminal.sentSize);

const sizeClaimed = (terminal: TerminalContext): TerminalContext => ({
  ...terminal,
  sendingSize: terminal.size,
});

const resizeOf = (terminal: TerminalContext): Resize => ({
  cols: terminal.sendingSize.cols,
  rows: terminal.sendingSize.rows,
  terminal: terminal.id,
});

const sizeSettled = (terminal: TerminalContext): TerminalContext => ({
  ...terminal,
  sentSize: terminal.sendingSize,
});

const wasOpened = (outcome: OpenOutcome): boolean => outcome.kind === "opened";

const gaveOutput = (outcome: ReadOutcome): boolean => outcome.kind === "output";

const sawExit = (outcome: ReadOutcome): boolean =>
  outcome.kind === "output" && outcome.exited !== null;

const bytesOf = (outcome: ReadOutcome): Uint8Array =>
  outcome.kind === "output" ? outcome.bytes : new Uint8Array();

export {
  INITIAL_TERMINAL,
  NO_TERMINAL_KEY,
  bytesOf,
  gaveOutput,
  hasTyped,
  inputOf,
  openingOf,
  readingOf,
  resizeOf,
  sawExit,
  sizeClaimed,
  sizeDiffers,
  sizeSettled,
  startsAfresh,
  terminalFailed,
  terminalFitted,
  terminalOpened,
  terminalRead,
  terminalRestarted,
  terminalShown,
  terminalTyped,
  typedClaimed,
  typedSettled,
  wasOpened,
};
export type {
  TerminalCells,
  TerminalContext,
  TerminalFeed,
  TerminalOwner,
  TerminalProblem,
  TerminalShow,
};
