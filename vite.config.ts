import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

// Kestrel build config: two entries (launcher + settings) sharing one bundle.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5273,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "chrome105",
    minify: "esbuild",
    sourcemap: false,
    rollupOptions: {
      input: {
        launcher: resolve(__dirname, "index.html"),
        settings: resolve(__dirname, "settings.html"),
      },
      output: {
        manualChunks: {
          vendor: ["react", "react-dom"],
        },
      },
    },
  },
});
