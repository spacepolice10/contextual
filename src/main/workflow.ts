import type { FlowContent, TaskPlanGroup } from "../diagrams"

export function flattenPlan(groups: TaskPlanGroup[]) {
  let stage = 0
  let position = 0
  return groups.flatMap(({ mode, tasks }) => {
    const ordered = tasks.map((name) => ({
      name, position: position++, executionStage: mode === "parallel" ? stage : stage++,
    }))
    if (mode === "parallel") stage++
    return ordered
  })
}

export function taskOrder(a: FlowContent, b: FlowContent) {
  return (a.position ?? a.id) - (b.position ?? b.id) || a.id - b.id
}

// Null means blocked. An empty array means ready with no prerequisites.
// The visual node hierarchy never participates in execution decisions.
export function prerequisiteTasks(all: FlowContent[], task: FlowContent): FlowContent[] | null {
  const byId = new Map(all.map((item) => [item.id, item]))
  const children = new Map<number, FlowContent[]>()
  for (const item of all) {
    if (item.kind !== "task" || item.parentId === null) continue
    const list = children.get(item.parentId) ?? []
    list.push(item)
    children.set(item.parentId, list)
  }
  const prerequisites = new Map<number, FlowContent>()
  function collect(item: FlowContent, seen: Set<number>): boolean {
    if (seen.has(item.id) || item.status !== "accepted") return false
    seen.add(item.id)
    for (const child of children.get(item.id) ?? []) {
      if (!collect(child, seen)) return false
    }
    prerequisites.set(item.id, item)
    return true
  }

  const ancestors = new Set<number>()
  let current = task
  while (current.kind === "task") {
    if (ancestors.has(current.id) || current.parentId === null) return null
    ancestors.add(current.id)
    const parent = byId.get(current.parentId)
    if (!parent) return null
    if (parent.kind === "goal" ? parent.status !== "approved" :
      parent.kind !== "task" || (parent.status !== "queued" && parent.status !== "accepted")) return null
    for (const sibling of (children.get(parent.id) ?? []).sort(taskOrder)) {
      if ((sibling.executionStage ?? sibling.position ?? sibling.id) < (current.executionStage ?? current.position ?? current.id)) {
        if (!collect(sibling, new Set())) return null
      }
    }
    if (parent.kind === "goal") return [...prerequisites.values()]
    current = parent
  }
  return null
}
