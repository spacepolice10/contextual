import { readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js"
import type { CallToolResult } from "@modelcontextprotocol/sdk/types.js"
import { goalPlanSchema, taskResultSchema } from "../agents/work-schema"

export function registerAgentTools(server: McpServer, directory: string, mode: "plan-goal" | "work-task") {
  const context = readFileSync(join(directory, "context.json"), "utf8")
  const planning = mode === "plan-goal"
  server.registerTool(planning ? "get_goal" : "get_task", {
    description: "Read the Contextual work item selected for this run.",
    annotations: { readOnlyHint: true, openWorldHint: false },
  }, () => ({ content: [{ type: "text", text: context }] }))

  server.registerTool(planning ? "submit_goal_plan" : "submit_task_result", {
    description: "Submit one completed proposal for the work item in this run.",
    inputSchema: planning ? goalPlanSchema : taskResultSchema,
    annotations: { destructiveHint: false, openWorldHint: false },
  }, (response: unknown): CallToolResult => {
    try {
      writeFileSync(join(directory, "response.json"), JSON.stringify(response), { mode: 0o600, flag: "wx" })
      return { content: [{ type: "text", text: "Response received." }] }
    } catch {
      return { isError: true, content: [{ type: "text", text: "The response could not be saved or was already submitted." }] }
    }
  })

}
