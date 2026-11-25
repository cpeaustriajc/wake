import { defineConfig } from "electron-vite";
import solid from "@solidjs/vite-plugin";

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
    plugins: [solid()],
  },
});
