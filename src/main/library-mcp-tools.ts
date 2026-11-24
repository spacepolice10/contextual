import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js"
import { z } from "zod"
import { goalTextLimit, workflowActions } from "../diagrams"
import { applyWorkflow, createContent, createFlow, createFlowNode, placeContent, readLibrary, renameFlow, updateContent, updateFlowNode } from "./database"

const id = z.number().int().positive()
const diagramName = z.string().trim().min(1).max(160)
const nodeName = z.string().trim().min(1).max(goalTextLimit)
const kind = z.enum(["goal", "task", "comment", "solution", "evidence"])
const body = z.string().trim().max(16000)
const readOnly = { readOnlyHint: true, openWorldHint: false }
const edit = { destructiveHint: false, openWorldHint: false }

function result(value: unknown) {
  return { content: [{ type: "text" as const, text: JSON.stringify(value) }] }
}

export function registerLibraryTools(server: McpServer, path: string, canEdit: boolean) {
  server.registerTool("list_diagrams", {
    description: "List Contextual diagrams and flows by ID and name without their content.",
    annotations: readOnly,
  }, async () => result((await readLibrary(path)).map(({ id, name, flow }) => ({ id, name, type: flow ? "flow" : "diagram" }))))

  server.registerTool("get_diagram", {
    description: "Read a flow and its nodes, or the Mermaid source of a standalone diagram.",
    inputSchema: z.strictObject({ diagramId: id }),
    annotations: readOnly,
  }, async ({ diagramId }) => {
    const [diagram] = await readLibrary(path, diagramId)
    if (!diagram) throw new Error("Diagram not found.")
    return result(diagram)
  })

  if (!canEdit) return

  server.registerTool("create_flow", {
    description: "Create a flow with a root node named after the flow.",
    inputSchema: z.strictObject({ name: diagramName }),
    annotations: edit,
  }, async (input) => result(await createFlow(path, input.name)))

  server.registerTool("rename_flow", {
    description: "Rename an existing flow and its root.",
    inputSchema: z.strictObject({ flowId: id, name: diagramName }),
    annotations: { ...edit, idempotentHint: true },
  }, async ({ flowId, name }) => result(await renameFlow(path, flowId, name)))

  server.registerTool("create_node", {
    description: "Add a goal under the flow root or add a task, comment, solution or evidence beneath a goal or another branch node.",
    inputSchema: z.strictObject({ flowId: id, parentId: id, kind, name: nodeName }),
    annotations: edit,
  }, async (input) => result(await createFlowNode(path, input.flowId, input.parentId, input.kind, input.name)))

  server.registerTool("update_node", {
    description: "Rename a node's content, move its visual branch independently of content relationships, or do both.",
    inputSchema: z.strictObject({
      flowId: id, nodeId: id, name: nodeName.optional(), parentId: id.optional(),
    }).refine((input) => input.name !== undefined || input.parentId !== undefined, "Provide a name or parentId."),
    annotations: { ...edit, idempotentHint: true },
  }, async ({ flowId, nodeId, ...changes }) => result(await updateFlowNode(path, flowId, nodeId, changes)))

  server.registerTool("create_content", {
    description: "Create a content record without placing it on the diagram. parentId refers to content, not a visual node.",
    inputSchema: z.strictObject({ flowId: id, kind, name: nodeName, body: body.optional(), parentId: id.optional() }),
    annotations: edit,
  }, async ({ flowId, ...content }) => result(await createContent(path, flowId, content)))

  server.registerTool("update_content", {
    description: "Edit a content record. All visual placements keep their IDs and reflect the update.",
    inputSchema: z.strictObject({ flowId: id, contentId: id, name: nodeName.optional(), body: body.optional() })
      .refine((input) => input.name !== undefined || input.body !== undefined, "Provide name or body."),
    annotations: { ...edit, idempotentHint: true },
  }, async ({ flowId, contentId, ...changes }) => result(await updateContent(path, flowId, contentId, changes)))

  server.registerTool("place_content", {
    description: "Add a visual node referencing one or more existing content records. parentId is a visual node ID.",
    inputSchema: z.strictObject({
      flowId: id, parentId: id,
      contentIds: z.array(id).min(1).max(20).refine((ids) => new Set(ids).size === ids.length, "IDs must be distinct."),
    }),
    annotations: edit,
  }, async ({ flowId, parentId, contentIds }) => result(await placeContent(path, flowId, parentId, contentIds)))

  server.registerTool("apply_workflow", {
    description: "Apply an explicitly requested workflow action to content. Approving a plan or retrying a task queues work for Codex while the app runs.",
    inputSchema: z.strictObject({ flowId: id, contentId: id, action: z.enum(workflowActions) }),
    annotations: { ...edit, openWorldHint: true },
  }, async ({ flowId, contentId, action }) => {
    await applyWorkflow(path, flowId, action, contentId)
    return result({ applied: true })
  })
}
