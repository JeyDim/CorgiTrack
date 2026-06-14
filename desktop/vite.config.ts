import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @tauri-apps/cli задаёт TAURI_DEV_HOST при работе на устройстве в локалке.
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],

  // Tauri ждёт фиксированный порт и не должен падать, если порт занят.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // не следим за Rust-исходниками — это делает сам tauri.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Переменные окружения, доступные фронтенду.
  envPrefix: ["VITE_", "TAURI_ENV_"],
});
