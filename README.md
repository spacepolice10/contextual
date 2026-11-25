# Contextual

A desktop workspace for local workflows. SQLite stores the flow nodes and content records. Contextual generates Mermaid from those records for Obsidian’s Pele.

## Run locally

Use Node.js 22.16 or later in the 22.x line, or Node.js 24 or later.

```sh
npm ci
npm start
```

On first launch Contextual creates `diagrams.sqlite` under Electron’s `userData` directory and seeds example flows. Each sidebar link opens a diagram at `/diagrams/:id`. Browser history and reload retain the route.

```sh
npm run typecheck
npm run build
npm run make
```

Forge writes the packaged desktop app and ZIP into `out/`.

Drizzle ORM handles the library queries over Electron's bundled `node:sqlite`. Schema migrations preserve existing diagram and node IDs. The Flow migration converts the original samples and keeps custom Mermaid source in a result record. Flow content has its own IDs and parent relationships. A placement can reference several records, and the same record can appear in multiple placements.

## Codex and Contextual MCP

Install Codex CLI and sign in with `codex login`. Inside a flow, click `+` on the root and enter a goal. Contextual saves the goal and asks Codex for a plan through local stdio MCP. Open the goal or task list and choose **Принять план и запустить Codex** to start the queued tasks. Contextual runs up to two tasks at a time and puts each result, comment, and supporting item in its task branch. Open a result to read its full text and choose **Принять результат**. Failed tasks can be retried from their details.

These built-in Codex runs produce text drafts and specifications. They have scoped access to the current goal or task, with shell and web tools disabled. Contextual resumes interrupted tasks when the app reopens.

To connect a separate Codex session, launch Contextual once to initialize its database, then run the following from the development checkout. Replace the database path with the one under your Electron `userData` directory.

```sh
codex mcp add contextual -- node "$PWD/.vite/build/contextual-mcp.cjs" read "/absolute/path/to/diagrams.sqlite"
```

Use `read` to list diagrams and inspect flows. Choose `edit` when you want Codex to create and rename flows, add goals, tasks, comments, solutions or evidence, and edit or move branches. The server uses the selected database and validates each operation.

`create_content` and `update_content` work on content records. `place_content` places existing records on the diagram, including several records in one node. `apply_workflow` supports the same plan approval, retry, and result acceptance actions as the app.

For a packaged macOS build, point Codex at the installed app and its bundled server.

```sh
app_bundle="/absolute/path/to/contextual.app"
codex mcp add contextual --env ELECTRON_RUN_AS_NODE=1 -- \
  "$app_bundle/Contents/MacOS/contextual" \
  "$app_bundle/Contents/Resources/app.asar/.vite/build/contextual-mcp.cjs" \
  read "/absolute/path/to/diagrams.sqlite"
```

## Frontend guidance

The repository includes upstream project skills for [nanotags components](https://github.com/psd-coder/nanotags/tree/main/packages/nanotags/skills/nanotags-components), the [Evil Martians design system workflow](https://github.com/ymandrikov/ai-design-system/tree/main/skills/design-system), and [Evil Martians’ Tailwind CSS practices](https://github.com/evilmartians/agent-skills/tree/main/skills/tailwind-best-practices). The app styles use plain CSS.
