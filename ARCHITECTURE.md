# Modular architecture: first extraction

The shared UI selects the IPC or HTTP client at bootstrap. Application/library.ts does not import Electron or Codex. Hono serves the same browser bundle with an agent-free API and binds loopback only; no authentication, do not expose publicly.

Creating goals and approving plans no longer automatically triggers agents. Desktop planning and task execution are explicit. Existing queued work can still run in desktop. Manual child creation is available in both clients. HTTP rejects agent execution actions. The worker receives an executor.

Run npm install (lockfile needs regeneration), npm run typecheck, npm run web; open http://127.0.0.1:3000. Desktop remains npm start. The default web database is separate.

Remaining: move storage out of main after merging concurrent schema changes; consolidate all MCP CRUD through application services; replace file watching/polling with revisions; separate execution attempts into AgentsWork; inject MCP launcher configuration; add auth/sync before public deployment. Focused View is a separate branch.
