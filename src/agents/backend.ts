export type AgentsMcpLaunch = {
  command: string
  args: string[]
  env?: Record<string, string>
}
export type AgentsRequest = {
  directory: string
  prompt: string
  mcp: AgentsMcpLaunch
  tools: string[]
  signal: AbortSignal
}
export interface AgentsBackend {
  readonly id: string
  execute(request: AgentsRequest): Promise<void>
}
export class AgentsBackendRegistry {
  private readonly backends = new Map<string, AgentsBackend>()
  constructor(backends: AgentsBackend[]) {
    for (const backend of backends) {
      if (this.backends.has(backend.id)) throw new Error("Duplicate agents backend: " + backend.id)
      this.backends.set(backend.id, backend)
    }
  }
  get(id: string): AgentsBackend {
    const backend = this.backends.get(id)
    if (!backend) throw new Error("Unknown agents backend: " + id)
    return backend
  }
  list(): string[] { return [...this.backends.keys()] }
}
