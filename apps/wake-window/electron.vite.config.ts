import stylex from "@stylexjs/unplugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "electron-vite";
import type { Plugin } from "vite";

const isPlugin = (candidate: unknown): candidate is Plugin =>
  typeof candidate === "object" && candidate !== null && "name" in candidate;

const stylexPlugin: unknown = stylex({
  aliases: { "#/*": [`${import.meta.dirname}/src/*`] },
});

export default defineConfig({
  main: {},
  preload: {
    build: {
      rolldownOptions: {
        output: { format: "cjs" },
      },
    },
  },
  renderer: {
    optimizeDeps: { exclude: ["@ui-sdk/core"] },
    plugins: [...(isPlugin(stylexPlugin) ? [stylexPlugin] : []), react()],
  },
});
