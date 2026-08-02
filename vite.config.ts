import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // El webview de producción de Tauri no sirve los archivos como un servidor web con raíz "/" —
  // los intercepta vía su propio protocolo interno. Con la base por defecto ("/"), index.html
  // queda con rutas absolutas ("/assets/...") que ese protocolo no resuelve, y el build
  // empaquetado (.deb/.AppImage/.msi/.dmg) arranca en blanco. Con rutas relativas sí funciona,
  // y no afecta a `tauri dev` (Vite las sigue sirviendo igual desde su propio servidor).
  base: './',

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}))
