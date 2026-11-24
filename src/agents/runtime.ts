import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import type { AgentsBackend, AgentsMcpLaunch } from "./backend"
import { goalPlanSchema, taskResultSchema } from "./work-schema"

export type AgentsWorkMode = "plan-goal" | "work-task"
export type AgentsMcpFactory = (mode: AgentsWorkMode, directory: string) => AgentsMcpLaunch

export class AgentsRuntime {
  constructor(private readonly backend: AgentsBackend, private readonly createMcp: AgentsMcpFactory) {}
  private async run(mode: AgentsWorkMode, context: object, tools: string[], prompt: string, signal: AbortSignal): Promise<unknown> {
    signal.throwIfAborted()
    const directory = await mkdtemp(join(tmpdir(), "contextual-agent-"))
    try {
      await writeFile(join(directory, "context.json"), JSON.stringify(context), { mode: 0o600 })
      await this.backend.execute({ directory, prompt, tools, signal, mcp: this.createMcp(mode, directory) })
      signal.throwIfAborted()
      try {
        return JSON.parse(await readFile(join(directory, "response.json"), "utf8"))
      } catch (error) {
        throw new Error("Agent did not submit a valid result through Contextual MCP.", { cause: error })
      }
    } finally {
      await rm(directory, { recursive: true, force: true })
    }
  }
  async generateGoalTasks(context: { flow: string; goal: string; details: string | null }, signal: AbortSignal) {
    const response = await this.run("plan-goal", context, ["get_goal", "submit_goal_plan"],
      "Read get_goal for the desired outcome. Propose an ordered list of distinct, actionable tasks in the goal language. Respect any explicit variant count and assign each requested variant its own task with an identifiable angle. Only plan. Call submit_goal_plan once with the complete list.", signal)
    return goalPlanSchema.parse(response).tasks
  }
  
  async completeTask(context: {
    flow: string; goal?: string; goalDetails?: string | null; parentTask?: string
    task: string; details: string | null; siblingTasks: string[]
  }, signal: AbortSignal) {
    const response = await this.run("work-task", context, ["get_task", "submit_task_result"],
      "Read get_task, then carry out that single task in the goal language. Produce the actual draft, recommendations, or usable specification in the solution body. For work that requires files, visual rendering, external data or tools you lack, deliver the part you can do here and clearly state what is pending. Distinguish this variant from its siblings. Add a comment only when it helps review. Include evidence only for actual, reproducible checks or concrete references, and never invent them. Call submit_task_result once with the finished contribution.", signal)
    return taskResultSchema.parse(response)
  }
}
