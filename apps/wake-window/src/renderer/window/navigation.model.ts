import { m } from "@earnestjs/model";

const Key = m.enum(["a1", "a2"]);

export const Navigation = m.model("Navigation", {
  daemon: m.set(Key),
  fleet: m.set(Key),
  navigation: m.enum(["floor", "workspace"]).initial("floor"),
  opened: Key.optional().initial(null),
});

export const workspaceOpen = m.action(
  Navigation,
  "a tile is clicked and its workspace opens",
  {
    changes: (_, { key }) => ({ navigation: "workspace", opened: key }),
    input: { key: Key },
    when: (s, { key }) => s.navigation === "floor" && s.fleet.has(key),
  }
);

export const workspaceClose = m.action(
  Navigation,
  "the breadcrumb leads back to the floor",
  {
    changes: () => ({ navigation: "floor", opened: null }),
    when: (s) => s.navigation === "workspace",
  }
);

export const agentJoins = m.action(Navigation, "the daemon gains an agent", {
  changes: (s, { key }) => ({ daemon: s.daemon.add(key) }),
  input: { key: Key },
  when: (s, { key }) => !s.daemon.has(key),
});

export const agentLeaves = m.action(Navigation, "the daemon loses an agent", {
  changes: (s, { key }) => ({ daemon: s.daemon.delete(key) }),
  input: { key: Key },
  when: (s, { key }) => s.daemon.has(key),
});

const stays = (s) => s.opened !== null && s.daemon.has(m.some(s.opened));

export const fleetRead = m.action(Navigation, "a read of the fleet lands", {
  changes: (s) => ({
    fleet: s.daemon,
    navigation: stays(s) ? s.navigation : "floor",
    opened: stays(s) ? s.opened : null,
  }),
  when: (s) => !s.fleet.isSubset(s.daemon) || !s.daemon.isSubset(s.fleet),
});

export const neverOpenForAbsent = m.rule(
  Navigation,
  "The workspace is never open for an agent that left the fleet",
  (s) =>
    s.navigation !== "workspace" ||
    (s.opened !== null && s.fleet.has(m.some(s.opened)))
);

export const openedOnlyInWorkspace = m.rule(
  Navigation,
  "An agent is opened exactly while the workspace shows",
  (s) => (s.navigation === "workspace") === (s.opened !== null)
);

export const onTheFloor = m.end(
  Navigation,
  "The floor shows and matches the daemon",
  (s) => s.navigation === "floor"
);
