export interface Diagram {
  id: number
  name: string
  source: string
  flow?: Flow
}

export interface Flow {
  nodes: FlowNode[]
  contents: FlowContent[]
  versions: ContentVersion[]
  agentsWork: AgentsWork[]
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
  position: number | null
  executionStage: number | null
  latestVersionId: number | null
  selectedVersionId: number | null
}

export type ExecutionMode = "sequential" | "parallel"

export interface TaskPlanGroup {
  mode: ExecutionMode
  tasks: string[]
}

export interface ContentVersion {
  id: number
  contentId: number
  parentVersionId: number | null
  agentsWorkId: number | null
  name: string
  body: string | null
  createdAt: string
}

export type AgentsWorkKind = "planning" | "execution" | "revision"
export type AgentsWorkStatus = "queued" | "running" | "completed" | "failed" | "cancelled" | "interrupted"

export interface AgentsWork {
  id: number
  contentId: number
  kind: AgentsWorkKind
  status: AgentsWorkStatus
  agent: string
  input: string | null
  instruction: string | null
  error: string | null
  retryOfId: number | null
  baseVersionId: number | null
  resultContentId: number | null
  createdAt: string
  startedAt: string | null
  finishedAt: string | null
}

export interface TaskContext {
  flow: string
  goal?: string
  goalDetails?: string | null
  parentTask?: string
  task: string
  details: string | null
  siblingTasks: string[]
  prerequisites: { taskId: number; task: string; resultId: number; versionId: number; name: string; body: string | null }[]
  revision?: { resultId: number; versionId: number; name: string; body: string | null; instruction: string }
}

export interface ChangeEvent {
  id: number
  flowId: number
  entity: "flow" | "content" | "node" | "version" | "agents_work"
  entityId: number
  operation: "created" | "updated" | "deleted"
  createdAt: string
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
  get: (id: number) => Promise<Diagram | null>
  createGoal: (diagramId: number, text: string) => Promise<FlowNode>
  planGoal: (diagramId: number, goalId: number) => Promise<FlowNode[]>
  workflow: (diagramId: number, action: WorkflowAction, contentId: number) => Promise<void>
  onChange: (listener: (events: ChangeEvent[]) => void) => () => void
}

export const goalTextLimit = 2000
export const listDiagramsChannel = "diagrams:list"
export const getDiagramChannel = "diagrams:get"
export const createGoalChannel = "diagrams:createGoal"
export const planGoalChannel = "diagrams:planGoal"
export const diagramsChangedChannel = "diagrams:changed"
export const workflowChannel = "diagrams:workflow"
export const workflowActions = ["approve_plan", "accept_result", "retry_task"] as const
export type WorkflowAction = (typeof workflowActions)[number]
