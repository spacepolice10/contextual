import type { ContentStatus, Flow, FlowContent, WorkflowAction } from "../diagrams"
import type { FlowSelection } from "./flow"

export const statusLabel: Record<ContentStatus, string> = {
  draft: "Черновик",
  approved: "План принят",
  planned: "Запланирована",
  queued: "В очереди",
  running: "В работе",
  review: "На проверке",
  accepted: "Принято",
  failed: "Агент не вернул результат",
}

interface InspectorAction {
  type: WorkflowAction | "plan_goal"
  id: number
  label: string
}

export function inspectFlow(flow: Flow, selection: FlowSelection) {
  const contentById = new Map(flow.contents.map((content) => [content.id, content]))
  const items = selection.contentIds.map((id) => contentById.get(id)).filter((content) => content !== undefined)
  if (!items.length) return
  const single = items.length === 1 ? items[0] : undefined
  const parentId = items[0].parentId
  const parent = parentId === null ? undefined : contentById.get(parentId)
  const planParent = single?.kind === "goal" || single?.kind === "task" ? single
    : !single && (parent?.kind === "goal" || parent?.kind === "task")
      && items.every((item) => item.kind === "task" && item.parentId === parent.id) ? parent
    : undefined
  const children = (id: number) => flow.contents.filter((content) => content.parentId === id)
  const plan = planParent ? children(planParent.id).filter(({ kind }) => kind === "task") : []
  let action: InspectorAction | undefined
  let listed: FlowContent[] = items

  if (planParent && plan.some(({ status }) => status === "planned")) {
    action = { type: "approve_plan", id: planParent.id, label: "Принять план" }
  } else if (planParent?.kind === "goal" && !plan.length && selection.nodeId !== undefined) {
    action = { type: "plan_goal", id: selection.nodeId, label: "Составить план" }
  } else if (single?.kind === "solution" && single.status === "review") {
    action = { type: "accept_result", id: single.id, label: "Принять результат" }
  } else if (single?.kind === "task" && single.status === "approved") {
    action = { type: "queue_task", id: single.id, label: "Запустить агента" }
  } else if (single?.kind === "task" && single.status === "failed") {
    action = { type: "retry_task", id: single.id, label: "Повторить задачу" }
  }

  if (planParent && !single) listed = plan
  else if (single) listed = children(single.id)

  const title = single?.name ?? (planParent ? "План задач" : "Записи")
  return { title, body: single?.body, status: (single ?? planParent)?.status, listed, action }
}
