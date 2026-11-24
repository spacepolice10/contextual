import { spawn } from "node:child_process"
import { homedir } from "node:os"
import { delimiter, join } from "node:path"

export type AgentsCliRequest = {
  command: string
  args: string[]
  cwd: string
  signal: AbortSignal
  timeoutMs?: number
}

// Never invoke a shell or interpolate user input into a shell command.
export async function executeAgentsCli(request: AgentsCliRequest): Promise<void> {
  request.signal.throwIfAborted()
  await new Promise<void>((resolve, reject) => {
    const child = spawn(request.command, request.args, {
      cwd: request.cwd,
      env: {
        ...process.env,
        PATH: [process.env.PATH, "/opt/homebrew/bin", "/usr/local/bin", join(homedir(), ".local", "bin")].filter(Boolean).join(delimiter),
      },
      stdio: ["ignore", "ignore", "pipe"],
      windowsHide: true,
      shell: false,
    })
    let failure: Error | undefined
    let escalation: ReturnType<typeof setTimeout> | undefined
    const stop = (error: Error) => {
      if (failure) return
      failure = error
      child.kill("SIGTERM")
      escalation = setTimeout(() => child.kill("SIGKILL"), 3000)
      escalation.unref()
    }
    const abort = () => stop(new Error("Agent execution cancelled."))
    const timeout = setTimeout(() => stop(new Error("Agent execution timed out.")), request.timeoutMs ?? 120_000)
    const cleanup = () => {
      clearTimeout(timeout)
      if (escalation) clearTimeout(escalation)
      request.signal.removeEventListener("abort", abort)
    }
    // Drain stderr without exposing potentially sensitive agent output.
    child.stderr?.resume()
    child.once("error", error => { cleanup(); reject(error) })
    child.once("close", (code, signal) => {
      cleanup()
      if (failure) reject(failure)
      else if (code === 0) resolve()
      else reject(new Error("Agent CLI exited unsuccessfully (" + (code ?? signal) + ")."))
    })
    request.signal.addEventListener("abort", abort, { once: true })
    if (request.signal.aborted) abort()
  })
}
