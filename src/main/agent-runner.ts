import { execFile } from "node:child_process"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { homedir, tmpdir } from "node:os"
import { delimiter, join } from "node:path"
import { promisify } from "node:util"
import { goalPlanSchema, taskResultSchema } from "./goal-plan-schema"

const execFileAsync = promisify(execFile)

async function runAgent(
  mode: "plan-goal" | "work-task", context: object, tools: string[], prompt: string, signal: AbortSignal,
): Promise<unknown> {
  const directory = await mkdtemp(join(tmpdir(), "contextual-agent-"))
  try {
    await writeFile(join(directory, "context.json"), JSON.stringify(context), { mode: 0o600 })

    const mcp = "mcp_servers.contextual"
    const run = execFileAsync(process.env.CONTEXTUAL_CODEX_PATH || "codex", [
      "exec",
      "--ignore-user-config",
      "--ephemeral",
      "--skip-git-repo-check",
      "--sandbox", "read-only",
      "--disable", "shell_tool",
      "--disable", "hooks",
      "--disable", "plugins",
      "-c", 'approval_policy="never"',
      "-c", 'web_search="disabled"',
      "-c", `${mcp}.command=${JSON.stringify(process.execPath)}`,
      "-c", `${mcp}.args=${JSON.stringify([join(__dirname, "contextual-mcp.cjs"), mode, directory])}`,
      "-c", `${mcp}.env={ELECTRON_RUN_AS_NODE="1"}`,
      "-c", `${mcp}.required=true`,
      "-c", `${mcp}.enabled_tools=${JSON.stringify(tools)}`,
      "-c", `${mcp}.default_tools_approval_mode="approve"`,
      prompt,
    ], {
      cwd: directory,
      env: {
        ...process.env,
        PATH: [process.env.PATH, "/opt/homebrew/bin", "/usr/local/bin", join(homedir(), ".local", "bin")].filter(Boolean).join(delimiter),
      },
      timeout: 120_000,
      signal,
      maxBuffer: 512 * 1024,
      windowsHide: true,
    })
    run.child.stdin?.end()
    await run.catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") throw new Error("Codex CLI не найден. Установите его и войдите в аккаунт.")
      if ("killed" in error && error.killed) throw new Error("Codex не ответил вовремя. Попробуйте ещё раз.")
      throw new Error("Не удалось запустить Codex. Проверьте вход в Codex CLI и повторите попытку.")
    })

    try {
      return JSON.parse(await readFile(join(directory, "response.json"), "utf8"))
    } catch {
      throw new Error("Codex не передал результат. Попробуйте ещё раз.")
    }
  } finally {
    await rm(directory, { recursive: true, force: true }).catch(() => {})
  }
}

export async function generateGoalTasks(context: { flow: string; goal: string; details: string | null }, signal: AbortSignal) {
  const response = await runAgent("plan-goal", context, ["get_goal", "submit_goal_plan"],
    "Read get_goal for the desired outcome. Propose an ordered list of distinct, actionable tasks in the goal language. Respect any explicit variant count and assign each requested variant its own task with an identifiable angle. Only plan. Call submit_goal_plan once with the complete list.", signal)
  return goalPlanSchema.parse(response).tasks
}

export async function completeTask(context: {
  flow: string; goal?: string; goalDetails?: string | null; parentTask?: string
  task: string; details: string | null; siblingTasks: string[]
}, signal: AbortSignal) {
  const response = await runAgent("work-task", context, ["get_task", "submit_task_result"],
    "Read get_task, then carry out that single task in the goal language. Produce the actual draft, recommendations, or usable specification in the solution body. For work that requires files, visual rendering, external data or tools you lack, deliver the part you can do here and clearly state what is pending. Distinguish this variant from its siblings. Add a comment only when it helps review. Include evidence only for actual, reproducible checks or concrete references, and never invent them. Call submit_task_result once with the finished contribution.", signal)
  return taskResultSchema.parse(response)
}
