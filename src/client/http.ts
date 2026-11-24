import type { DiagramLibrary } from "../diagrams"
export function createHttpClient(base = "/api"): DiagramLibrary {
  async function request<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(base + path, body === undefined ? undefined : { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) })
    if (!response.ok) throw new Error((await response.json().catch(() => ({}))).error ?? response.statusText)
    return response.json() as Promise<T>
  }
  return {
    list: () => request("/diagrams"),
    createGoal: (diagramId, text) => request("/goals", { diagramId, text }),
    createNode: (diagramId, parentId, kind, name) => request("/nodes", { diagramId, parentId, kind, name }),
    workflow: async (diagramId, action, contentId) => { await request("/workflow", { diagramId, action, contentId }) },
    onChange: listener => { const timer = setInterval(listener, 2000); return () => clearInterval(timer) },
  }
}
