import type { AgentsBackend, AgentsRequest } from "../backend"
import { executeAgentsCli } from "../transports/cli"

export class OpenAIBackend implements AgentsBackend {
  readonly id = "openai"
  constructor(private readonly executable = "codex") {}
  async execute(request: AgentsRequest): Promise<void> {
    const { mcp, tools, directory, prompt, signal } = request
    const key = "mcp_servers.contextual"
    const env = Object.entries(mcp.env ?? {}).map(([k, v]) => JSON.stringify(k) + "=" + JSON.stringify(v)).join(",")
    try {
      await executeAgentsCli({
        command: this.executable,
        args: [
          "exec", "--ignore-user-config", "--ephemeral", "--skip-git-repo-check",
          "--sandbox", "read-only", "--disable", "shell_tool", "--disable", "hooks", "--disable", "plugins",
          "-c", 'approval_policy="never"', "-c", 'web_search="disabled"',
          "-c", key + ".command=" + JSON.stringify(mcp.command),
          "-c", key + ".args=" + JSON.stringify(mcp.args),
          "-c", key + ".env={" + env + "}",
          "-c", key + ".required=true",
          "-c", key + ".enabled_tools=" + JSON.stringify(tools),
          "-c", key + '.default_tools_approval_mode="approve"',
          prompt,
        ],
        cwd: directory, signal,
      })
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        throw new Error("Codex CLI not found. Install it and sign in.", { cause: error })
      }
      throw error
    }
  }
}
