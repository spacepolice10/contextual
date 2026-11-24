import * as db from "../main/database"
import { z } from "zod"
import { goalTextLimit, workflowActions, type DiagramLibrary, type FlowNode } from "../diagrams"
export const entityId = z.number().int().positive().max(Number.MAX_SAFE_INTEGER)
export const nodeName = z.string().trim().min(1).max(goalTextLimit)
export const contentKind = z.enum(["goal", "task", "comment", "solution", "evidence"])
export const workflowAction = z.enum(workflowActions)
export type LibraryCommands = Omit<DiagramLibrary, "onChange" | "planGoal">
export function createLibrary(path: string): LibraryCommands {
  db.initializeLibrary(path)
  return {
    list: () => db.readLibrary(path),
    createGoal: async (id, text) => db.createGoal(path, entityId.parse(id), nodeName.parse(text)),
    createNode: async (id, parent, kind, name) => db.createFlowNode(path, entityId.parse(id), entityId.parse(parent), contentKind.parse(kind), nodeName.parse(name)),
    workflow: async (id, action, content) => db.applyWorkflow(path, entityId.parse(id), workflowAction.parse(action), entityId.parse(content)),
  }
}
export function createPlanningService(path: string, generate: (context: { flow: string; goal: string; details: string | null }, signal: AbortSignal) => Promise<string[]>, signal: AbortSignal) {
  const active = new Map<string, Promise<FlowNode[]>>()
  return async (diagramId: number, goalId: number) => {
    entityId.parse(diagramId); entityId.parse(goalId)
    const key = `${diagramId}:${goalId}`
    const existing = active.get(key)
    if (existing) return existing
    const plan = (async () => {
      const { context, hasTasks } = await db.readGoal(path, diagramId, goalId)
      return hasTasks ? [] : db.saveGoalTasks(path, diagramId, goalId, await generate(context, signal))
    })().finally(() => active.delete(key))
    active.set(key, plan)
    return plan
  }
}
