import { statSync } from "node:fs"
import { isAbsolute } from "node:path"
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js"
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js"
import { registerAgentTools } from "./agent-mcp-tools"
import { registerLibraryTools } from "./library-mcp-tools"

async function start() {
  const [mode, path, extra] = process.argv.slice(2)
  if (extra || !path || !isAbsolute(path)) throw new Error("Expected an access mode and an absolute path.")

  const server = new McpServer({ name: "contextual", version: "1.0.0" })
  if (mode === "plan-goal" || mode === "work-task") {
    registerAgentTools(server, path, mode)
  } else if (mode === "read" || mode === "edit") {
    if (!statSync(path).isFile()) throw new Error("Contextual library file not found.")
    registerLibraryTools(server, path, mode === "edit")
  } else {
    throw new Error("Unknown Contextual MCP access mode.")
  }
  await server.connect(new StdioServerTransport())
}

void start().catch((error) => {
  console.error(error instanceof Error ? error.message : "Could not start Contextual MCP.")
  process.exitCode = 1
})
