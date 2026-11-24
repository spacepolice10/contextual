import { Hono } from "hono"
import { z } from "zod"
import { entityId, nodeName, contentKind, workflowAction, type LibraryCommands } from "../application/library"
export function createApi(library: LibraryCommands) {
  const app = new Hono()
  app.onError((error, c) => c.json({ error: error instanceof z.ZodError ? "Invalid input" : error.message }, 400))
  app.get("/diagrams", async c => c.json(await library.list()))
  app.post("/goals", async c => {
    const { diagramId, text } = z.object({ diagramId: entityId, text: nodeName }).parse(await c.req.json())
    return c.json(await library.createGoal(diagramId, text), 201)
  })
  app.post("/nodes", async c => {
    const input = z.object({ diagramId: entityId, parentId: entityId, kind: contentKind, name: nodeName }).parse(await c.req.json())
    return c.json(await library.createNode(input.diagramId, input.parentId, input.kind, input.name), 201)
  })
  app.post("/workflow", async c => {
    const input = z.object({ diagramId: entityId, contentId: entityId, action: workflowAction }).parse(await c.req.json())
    if (input.action === "queue_task" || input.action === "retry_task") return c.json({ error: "Agent execution unavailable" }, 403)
    await library.workflow(input.diagramId, input.action, input.contentId)
    return c.json({ applied: true })
  })
  return app
}
