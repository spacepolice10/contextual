# Modular architecture: first extraction

The shared UI selects the IPC or HTTP client at bootstrap. Application/library.ts does not import Electron or any agent provider. Hono serves the same browser bundle with an agent-free API and binds loopback only; no authentication, do not expose publicly.

Creating goals and approving plans no longer automatically triggers agents. Desktop planning and task execution are explicit. Existing queued work can still run in desktop. Manual child creation is available in both clients. HTTP rejects agent execution actions. The worker receives an executor.

Run npm install (lockfile needs regeneration), npm run typecheck, npm run web; open http://127.0.0.1:3000. Desktop remains npm start. The default web database is separate.

Remaining: move storage out of main after merging concurrent schema changes; consolidate all MCP CRUD through application services; replace file watching/polling with revisions; separate execution attempts into AgentsWork; add auth/sync before public deployment. Focused View is a separate branch.

## Agents runtime

`src/agents` is independent of Electron. `AgentsRuntime` owns scoped MCP work context, validation and cleanup; `AgentsBackend` owns the provider protocol; `AgentsBackendRegistry` selects one backend for the application; `executeAgentsCli` launches a local executable without a shell. The desktop composition root supplies the MCP launch descriptor. Hono remains agent-free. OpenAI (via Codex CLI) is the only implemented backend; no per-task provider selection, cross-provider session migration or orchestration. The MCP contract is one submission per invocation, not persistent sessions. Existing queue behavior is unchanged. The separate workflow-architecture branch owns AgentsWork persistence; merge it separately.
