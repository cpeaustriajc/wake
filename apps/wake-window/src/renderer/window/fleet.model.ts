import { m } from "@earnestjs/model";

const Reading = m.enum(["agents", "daemon_down", "unreadable"]);
const Count = m.int(0, 2);

export const Fleet = m.model("Fleet", {
  cycle: m.enum(["reading", "waiting"]).initial("reading"),
  daemonAgents: Count,
  lastGood: m.int(0, 2).initial(0),
  polls: m.int(0, 3).initial(0),
  shownAgents: m.int(0, 2).initial(0),
  status: m
    .enum(["connecting", "live", "daemonDown", "unreadable"])
    .initial("connecting"),
});

const statusOf = (reading) =>
  reading === "agents"
    ? "live"
    : reading === "daemon_down"
      ? "daemonDown"
      : "unreadable";

export const readDone = m.action(Fleet, "a read of the fleet finishes", {
  changes: (s, { reading }) => ({
    cycle: "waiting",
    lastGood: reading === "agents" ? s.daemonAgents : s.lastGood,
    shownAgents: reading === "agents" ? s.daemonAgents : s.shownAgents,
    status: statusOf(reading),
  }),
  input: { reading: Reading },
  when: (s) => s.cycle === "reading",
});

export const poll = m.action(Fleet, "the poll delay passes", {
  changes: (s) => ({ cycle: "reading", polls: s.polls + 1 }),
  when: (s) => s.cycle === "waiting" && s.polls < 3,
});

export const daemonChanges = m.action(
  Fleet,
  "the daemon's list of agents changes",
  {
    changes: (_, { count }) => ({ daemonAgents: count }),
    input: { count: Count },
    when: (s, { count }) => s.daemonAgents !== count,
  }
);

export const keepsLastAgents = m.rule(
  Fleet,
  "A failed read after a good one keeps the last agents",
  (s) => s.shownAgents === s.lastGood
);

export const connectingOnlyAtFirst = m.rule(
  Fleet,
  "The fleet is connecting only until its first read finishes",
  (s) => s.status !== "connecting" || (s.polls === 0 && s.cycle === "reading")
);

export const liveShowsTheDaemonsList = m.rule(
  Fleet,
  "Right after a good read the floor shows what the daemon listed",
  (s) => s.status !== "live" || s.shownAgents === s.lastGood
);

export const pollsSpent = m.end(
  Fleet,
  "Three polls have passed and the last read finished",
  (s) => s.polls === 3 && s.cycle === "waiting"
);

export const view = m.view(Fleet, {
  state: (s) => `${s.status}, ${s.cycle}, ${s.shownAgents} shown`,
  step: {
    daemonChanges: "daemon changes",
    poll: "poll",
    readDone: "read done",
  },
  title: "Fleet",
});
