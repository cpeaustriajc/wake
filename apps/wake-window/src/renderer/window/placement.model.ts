import { m } from "@earnestjs/model";

const Cell = m.int(0, 2);
const SLOT = 0;

export const Placement = m.model("Placement", {
  asked: m.boolean().initial(false),
  auto: m.boolean().initial(false),
  autoSends: m.int(0, 2).initial(0),
  daemonAt: Cell.optional(),
  drops: m.int(0, 2).initial(0),
  failureShown: m.boolean().initial(false),
  gesture: m.enum(["idle", "pressing", "dragging", "dropped"]).initial("idle"),
  heldAt: Cell.optional().initial(null),
  justRefused: m.boolean().initial(false),
  queued: m.boolean().initial(false),
  reads: m.int(0, 3).initial(0),
  save: m.enum(["idle", "saving", "couldNotPlace"]).initial("idle"),
  userSaves: m.int(0, 2).initial(0),
});

const agrees = (s) =>
  s.heldAt !== null && s.daemonAt !== null && s.heldAt === s.daemonAt;

const inHand = (s) => s.gesture === "pressing" || s.gesture === "dragging";

const fresh = (s) => s.daemonAt === null && !s.asked && !inHand(s);

export const fleetRead = m.action(Placement, "a read of the fleet lands", {
  changes: (s) => ({
    asked: s.asked || fresh(s),
    auto: fresh(s) ? true : s.auto,
    autoSends: fresh(s) ? s.autoSends + 1 : s.autoSends,
    heldAt: fresh(s) ? SLOT : agrees(s) ? null : s.heldAt,
    justRefused: fresh(s) ? false : s.justRefused,
    queued: s.queued || fresh(s),
    reads: s.reads + 1,
  }),
  when: (s) => s.reads < 3,
});

const free = (s) =>
  (s.gesture === "idle" || s.gesture === "dropped") && !s.queued;

export const tilePress = m.action(Placement, "the tile is pressed", {
  changes: () => ({
    failureShown: false,
    gesture: "pressing",
    justRefused: false,
  }),
  when: (s) => free(s) && s.drops < 2 && s.reads > 0,
});

export const pointerMove = m.action(
  Placement,
  "the pointer moves 5 px or more from the press",
  {
    changes: (_, { cell }) => ({ gesture: "dragging", heldAt: cell }),
    input: { cell: Cell },
    when: (s) => s.gesture === "pressing" || s.gesture === "dragging",
  }
);

const moved = (s) => s.heldAt !== null && !agrees(s);

export const pointerRelease = m.action(Placement, "the pointer is released", {
  changes: (s) =>
    s.gesture === "pressing"
      ? { gesture: "idle" }
      : {
          auto: false,
          drops: s.drops + 1,
          gesture: "dropped",
          heldAt: moved(s) ? s.heldAt : null,
          queued: moved(s),
          userSaves: moved(s) ? s.userSaves + 1 : s.userSaves,
        },
  when: (s) => s.gesture === "pressing" || s.gesture === "dragging",
});

export const tileClick = m.action(
  Placement,
  "the click after a drop is swallowed, or the grace delay passes",
  {
    changes: () => ({ gesture: "idle" }),
    when: (s) => s.gesture === "dropped",
  }
);

export const tileNudge = m.action(
  Placement,
  "Alt and an arrow key move the tile one step",
  {
    changes: (s, { cell }) => ({
      auto: false,
      drops: s.drops + 1,
      failureShown: false,
      heldAt: cell,
      justRefused: false,
      queued: true,
      userSaves: s.userSaves + 1,
    }),
    input: { cell: Cell },
    when: (s, { cell }) =>
      free(s) &&
      s.drops < 2 &&
      s.reads > 0 &&
      (s.daemonAt === null || s.daemonAt !== cell),
  }
);

export const saveStarts = m.action(Placement, "the next queued place is sent", {
  changes: () => ({ save: "saving" }),
  when: (s) => s.queued && s.save !== "saving",
});

export const savePlaced = m.action(Placement, "the daemon takes the place", {
  changes: (s) => ({
    daemonAt: s.heldAt,
    failureShown: false,
    queued: false,
    save: "idle",
  }),
  when: (s) => s.save === "saving",
});

export const saveRefused = m.action(Placement, "the daemon refuses the place", {
  changes: () => ({
    failureShown: true,
    heldAt: null,
    justRefused: true,
    queued: false,
    save: "couldNotPlace",
  }),
  when: (s) => s.save === "saving",
});

export const saveDaemonDown = m.action(
  Placement,
  "the daemon cannot be reached for the place",
  {
    changes: (s) => ({
      asked: s.auto ? false : s.asked,
      autoSends: s.auto ? s.autoSends - 1 : s.autoSends,
      failureShown: true,
      heldAt: null,
      justRefused: true,
      queued: false,
      save: "couldNotPlace",
    }),
    when: (s) => s.save === "saving",
  }
);

export const neverDraggedAndSaved = m.rule(
  Placement,
  "A tile is never both being dragged and being saved",
  (s) => !(inHand(s) && s.queued)
);

export const pressWithoutMovePlacesNothing = m.rule(
  Placement,
  "A press that never moves 5 px never places anything",
  (s) => s.userSaves <= s.drops
);

export const unplacedSentOnce = m.rule(
  Placement,
  "An unplaced agent is sent to the daemon at most once, not counting sends the daemon never received",
  (s) => s.autoSends <= 1
);

export const pendingHasAReason = m.rule(
  Placement,
  "A pending position is dropped only when the daemon agrees or refuses",
  (s) => s.heldAt !== null || s.gesture === "pressing" || !s.queued
);

export const refusedShowsDaemon = m.rule(
  Placement,
  "After a refused save the tile shows the daemon's position and the reason is shown",
  (s) => !s.justRefused || (s.heldAt === null && s.failureShown)
);

export const savingHasAPlace = m.rule(
  Placement,
  "A place is being sent only while one is pending",
  (s) => s.save !== "saving" || s.heldAt !== null
);

export const nothingInFlight = m.end(
  Placement,
  "No gesture is under way and no place is queued or being sent",
  (s) => s.gesture !== "pressing" && s.gesture !== "dragging" && !s.queued
);

export const view = m.view(Placement, {
  state: (s) => `${s.gesture}, ${s.save}`,
  step: {
    fleetRead: "fleet.read",
    pointerMove: "pointer.move",
    pointerRelease: "pointer.release",
    saveDaemonDown: "daemon down",
    savePlaced: "placed",
    saveRefused: "refused",
    saveStarts: "save starts",
    tileClick: "tile.click",
    tileNudge: "tile.nudge",
    tilePress: "tile.press",
  },
  title: "Placement",
});
