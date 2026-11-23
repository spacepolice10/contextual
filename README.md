# Contextual

A desktop viewer for local Mermaid diagrams. The diagram library is stored as Mermaid source in SQLite and rendered with Obsidian’s Pele.

## Run locally

Use Node.js 22.13 or later in the 22.x line, or Node.js 24 or later.

```sh
npm ci
npm start
```

On first launch Contextual creates `diagrams.sqlite` under Electron’s `userData` directory and seeds three examples. Each sidebar link opens a diagram at `/diagrams/:id`. Browser history and reload retain the route. The app exposes a read-only diagram API through Electron’s preload bridge.

```sh
npm run typecheck
npm run build
npm run make
```

Forge writes the packaged desktop app and ZIP into `out/`. The initial schema uses Electron’s bundled `node:sqlite` with one table and a versioned initialization transaction.

## Frontend guidance

The repository includes upstream project skills for [nanotags components](https://github.com/psd-coder/nanotags/tree/main/packages/nanotags/skills/nanotags-components), the [Evil Martians design system workflow](https://github.com/ymandrikov/ai-design-system/tree/main/skills/design-system), and [Evil Martians’ Tailwind CSS practices](https://github.com/evilmartians/agent-skills/tree/main/skills/tailwind-best-practices). The app styles use plain CSS.
