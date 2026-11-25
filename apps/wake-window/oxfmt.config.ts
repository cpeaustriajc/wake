import { defineConfig } from "oxfmt";
import ultracite from "ultracite/oxfmt";

export default defineConfig({
  ...ultracite,
  ignorePatterns: [
    ...(ultracite.ignorePatterns ?? []),
    "out/**",
    "src/renderer/keel/bundle.css",
    "src/renderer/keel/tokens.css",
  ],
});
