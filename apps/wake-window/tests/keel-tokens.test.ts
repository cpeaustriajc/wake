import { readFileSync, readdirSync } from "node:fs";

import { Array, Order, String } from "effect";
import { describe, expect, it } from "vitest";

const KEEL = new URL("../src/renderer/keel/", import.meta.url);
const SOURCES = new URL("../src/", import.meta.url);
const DECLARED = /^\s*(?<name>--[a-z0-9-]+):/gmu;
const REFERENCED = /var\((?<name>--[a-z0-9-]+)\)/gu;

const names = (text: string, pattern: RegExp): readonly string[] =>
  Array.sort(
    Array.dedupe(
      Array.map(
        Array.fromIterable(String.matchAll(pattern)(text)),
        (match) => match.groups?.["name"] ?? ""
      )
    ),
    Order.String
  );

describe("Keel's tokens", () => {
  it("are the same set in tokens.css and in the typed token module", () => {
    const stylesheet = readFileSync(new URL("tokens.css", KEEL), "utf-8");
    const tokens = readFileSync(
      new URL("keel-tokens.stylex.ts", KEEL),
      "utf-8"
    );
    expect(names(tokens, REFERENCED)).toStrictEqual(
      names(stylesheet, DECLARED)
    );
  });

  it("are the only stylesheets in the window", () => {
    const stylesheets = Array.sort(
      Array.filter(
        readdirSync(SOURCES, { encoding: "utf-8", recursive: true }),
        String.endsWith(".css")
      ),
      Order.String
    );
    expect(stylesheets).toStrictEqual([
      "renderer/keel/bundle.css",
      "renderer/keel/tokens.css",
    ]);
  });
});
