import { MakerZIP } from "@electron-forge/maker-zip"
import { VitePlugin } from "@electron-forge/plugin-vite"
import type { ForgeConfig } from "@electron-forge/shared-types"

const config: ForgeConfig = {
  packagerConfig: { asar: true },
  makers: [new MakerZIP()],
  plugins: [
    new VitePlugin({
      build: [
        { entry: "src/main/index.ts", config: "vite.main.config.ts", target: "main" },
        { entry: "src/main/contextual-mcp.ts", config: "vite.main.config.ts", target: "main" },
        { entry: "src/preload.ts", config: "vite.preload.config.ts", target: "preload" },
      ],
      renderer: [{ name: "main_window", config: "vite.renderer.config.ts" }],
    }),
  ],
}

export default config
