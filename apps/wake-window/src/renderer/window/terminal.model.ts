import { m } from "@earnestjs/model";

const Bytes = m.int(0, 3);
const Shells = m.int(0, 2);

export const Terminal = m.model("Terminal", {
  knownShell: Shells.initial(0),
  next: Bytes.initial(0),
  pane: m.enum(["hidden", "shown"]).initial("hidden"),
  reads: m.int(0, 2).initial(0),
  screen: Bytes.initial(0),
  shell: m.enum(["none", "running", "ended"]).initial("none"),
  shellCount: Shells.initial(0),
  terminal: m
    .enum(["closed", "opening", "open", "exited", "failed"])
    .initial("closed"),
  typed: m.int(0, 2).initial(0),
  written: Bytes.initial(0),
});

const startsShell = (s) => s.shell !== "running";

const afresh = (s) => startsShell(s) || s.knownShell !== s.shellCount;

export const terminalShow = m.action(
  Terminal,
  "the Terminal pane shows and its screen starts empty",
  {
    changes: () => ({
      knownShell: 0,
      next: 0,
      pane: "shown",
      reads: 0,
      screen: 0,
      terminal: "opening",
      typed: 0,
    }),
    when: (s) => s.pane === "hidden",
  }
);

export const terminalHide = m.action(
  Terminal,
  "the pane or the workspace is left",
  {
    changes: () => ({
      knownShell: 0,
      next: 0,
      pane: "hidden",
      reads: 0,
      screen: 0,
      terminal: "closed",
      typed: 0,
    }),
    when: (s) => s.pane === "shown",
  }
);

export const openDone = m.action(
  Terminal,
  "the daemon answers the open with the agent's shell, starting one if none runs",
  {
    changes: (s) => ({
      knownShell: startsShell(s) ? s.shellCount + 1 : s.shellCount,
      next: afresh(s) ? 0 : s.next,
      reads: s.reads + 1,
      screen: afresh(s) ? 0 : s.screen,
      shell: "running",
      shellCount: startsShell(s) ? s.shellCount + 1 : s.shellCount,
      terminal: "open",
      written: startsShell(s) ? 0 : s.written,
    }),
    when: (s) =>
      s.terminal === "opening" && (s.shell === "running" || s.shellCount < 2),
  }
);

export const openFails = m.action(
  Terminal,
  "the daemon refuses the open or cannot be reached",
  {
    changes: () => ({ terminal: "failed" }),
    when: (s) => s.terminal === "opening",
  }
);

export const shellWrites = m.action(Terminal, "the shell writes a byte", {
  changes: (s) => ({ written: s.written + 1 }),
  when: (s) => s.shell === "running" && s.written < 3,
});

export const shellExits = m.action(Terminal, "the shell exits", {
  changes: () => ({ shell: "ended" }),
  when: (s) => s.shell === "running",
});

export const readDone = m.action(
  Terminal,
  "a read comes back with the bytes past the last offset, or with the exit",
  {
    changes: (s) => ({
      next: s.written,
      reads: s.shell === "ended" ? 0 : s.reads,
      screen: s.screen + (s.written - s.next),
      terminal: s.shell === "ended" ? "exited" : "open",
      typed: s.shell === "ended" ? 0 : s.typed,
    }),
    when: (s) =>
      s.terminal === "open" &&
      s.reads > 0 &&
      (s.next < s.written || s.shell === "ended"),
  }
);

export const readFails = m.action(
  Terminal,
  "a read fails because the daemon is gone",
  {
    changes: () => ({ reads: 0, terminal: "failed", typed: 0 }),
    when: (s) => s.terminal === "open" && s.reads > 0,
  }
);

export const terminalRestart = m.action(
  Terminal,
  "Start a new shell or Try again is pressed",
  {
    changes: () => ({ terminal: "opening" }),
    when: (s) => s.terminal === "exited" || s.terminal === "failed",
  }
);

export const terminalType = m.action(Terminal, "a key is pressed", {
  changes: (s) => ({ typed: s.typed + 1 }),
  when: (s) => s.terminal === "open" && s.typed < 2,
});

export const writeSent = m.action(
  Terminal,
  "the waiting keystrokes are sent to the shell",
  {
    changes: () => ({ typed: 0 }),
    when: (s) => s.terminal === "open" && s.typed > 0,
  }
);

export const neverTwoReadLoops = m.rule(
  Terminal,
  "Never two read loops for one terminal",
  (s) => s.reads <= 1
);

export const leavingStopsReading = m.rule(
  Terminal,
  "Leaving the pane stops reading",
  (s) => s.reads === 0 || (s.pane === "shown" && s.terminal === "open")
);

export const bytesInOrder = m.rule(
  Terminal,
  "Bytes are appended in offset order with no gap or duplicate",
  (s) => s.screen === s.next
);

export const neverPastTheShell = m.rule(
  Terminal,
  "The screen never shows more than the shell wrote",
  (s) => s.knownShell !== s.shellCount || s.next <= s.written
);

export const writesOnlyWhileOpen = m.rule(
  Terminal,
  "No write is sent before open succeeded or after exit",
  (s) => s.typed === 0 || s.terminal === "open"
);

export const openIsTheCurrentShell = m.rule(
  Terminal,
  "An open terminal is the agent's running or just-ended shell",
  (s) => s.terminal !== "open" || s.knownShell === s.shellCount
);

export const paneHidden = m.end(
  Terminal,
  "The pane is hidden and nothing is read",
  (s) => s.pane === "hidden"
);

export const view = m.view(Terminal, {
  state: (s) => `${s.terminal}, shell ${s.shell}`,
  step: {
    openDone: "open done",
    openFails: "open fails",
    readDone: "read done",
    readFails: "read fails",
    shellExits: "shell exits",
    shellWrites: "shell writes",
    terminalHide: "terminal.hide",
    terminalRestart: "terminal.restart",
    terminalShow: "terminal.show",
    terminalType: "terminal.type",
    writeSent: "write sent",
  },
  title: "Terminal",
});
