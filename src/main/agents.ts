import { join } from "node:path"
import { AgentsBackendRegistry } from "../agents/backend"
import { CodexBackend } from "../agents/backends/codex"
import { AgentsRuntime } from "../agents/runtime"

// Electron only supplies the process launcher for its bundled MCP entrypoint.
export function createDesktopAgentsRuntime(): AgentsRuntime {
  const registry = new AgentsBackendRegistry([
    new CodexBackend(process.env.CONTEXTUAL_AGENTS_EXECUTABLE || "codex"),
  ])
  const backend = registry.get(process.env.CONTEXTUAL_AGENTS_BACKEND || "codex")
  return new AgentsRuntime(backend, (mode, directory) => ({
    command: process.execPath,
    args: [join(__dirname, "contextual-mcp.cjs"), mode, directory],
    env: { ELECTRON_RUN_AS_NODE: "1" },
  }))
}
