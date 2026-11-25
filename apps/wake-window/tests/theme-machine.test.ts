import { describe, expect, it } from "vitest";
import { createActor } from "xstate";

import {
  pickedIn,
  themeIn,
  themeMachine,
} from "#/renderer/theme/theme-machine.ts";

const started = (stored: string | null, systemDark: boolean) => {
  const actor = createActor(themeMachine, { input: { stored, systemDark } });
  actor.start();
  return actor;
};

describe("the theme machine", () => {
  it("follows the system until a theme is picked", () => {
    const actor = started(null, true);
    expect(themeIn(actor.getSnapshot())).toBe("dark");
    expect(pickedIn(actor.getSnapshot())).toBeNull();
    actor.send({ dark: false, type: "system.changed" });
    expect(themeIn(actor.getSnapshot())).toBe("light");
  });

  it("keeps a picked theme when the system changes", () => {
    const actor = started(null, false);
    actor.send({ type: "theme.toggled" });
    expect(pickedIn(actor.getSnapshot())).toBe("dark");
    actor.send({ dark: false, type: "system.changed" });
    expect(themeIn(actor.getSnapshot())).toBe("dark");
    actor.send({ type: "theme.toggled" });
    expect(themeIn(actor.getSnapshot())).toBe("light");
  });

  it("starts from the stored choice and ignores a value it does not know", () => {
    expect(themeIn(started("light", true).getSnapshot())).toBe("light");
    expect(pickedIn(started("sepia", true).getSnapshot())).toBeNull();
  });
});
