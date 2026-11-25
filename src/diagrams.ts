export interface Diagram {
  id: number
  name: string
  flow: Flow
}

export interface Flow {
  nodes: FlowNode[]
  contents: FlowContent[]
}

export type ContentKind = "goal" | "task" | "comment" | "solution" | "evidence"
export type ContentStatus = "draft" | "approved" | "planned" | "queued" | "running" | "review" | "accepted" | "failed"

export interface FlowContent {
  id: number
  parentId: number | null
  kind: ContentKind
  name: string
  body: string | null
  status: ContentStatus | null
}

export interface FlowNode {
  id: number
  parentId: number | null
  kind: "root" | ContentKind
  name: string
  contentIds: number[]
}

export interface DiagramLibrary {
  list: () => Promise<Diagram[]>
  createGoal: (diagramId: number, text: string) => Promise<FlowNode>
  planGoal: (diagramId: number, goalId: number) => Promise<FlowNode[]>
  workflow: (diagramId: number, action: WorkflowAction, contentId: number) => Promise<void>
  onChange: (listener: () => void) => () => void
}

export const goalTextLimit = 2000
export const listDiagramsChannel = "diagrams:list"
export const createGoalChannel = "diagrams:createGoal"
export const planGoalChannel = "diagrams:planGoal"
export const diagramsChangedChannel = "diagrams:changed"
export const workflowChannel = "diagrams:workflow"
export const workflowActions = ["approve_plan", "accept_result", "retry_task"] as const
export type WorkflowAction = (typeof workflowActions)[number]
