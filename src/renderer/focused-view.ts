import type { Flow, FlowNode } from "../diagrams"

/** Project the hierarchy without changing its logical IDs or source data. */
export function focusedFlow(flow: Flow, nodeId: number): Flow | undefined {
  if (!Number.isSafeInteger(nodeId) || nodeId < 1) return
  const root = flow.nodes.find((node) => node.id === nodeId)
  if (!root) return

  const children = new Map<number, FlowNode[]>()
  for (const node of flow.nodes) {
    if (node.parentId === null) continue
    const siblings = children.get(node.parentId) ?? []
    siblings.push(node)
    children.set(node.parentId, siblings)
  }
  const included = new Set<number>()
  const pending = [root]
  while (pending.length) {
    const node = pending.pop()!
    if (included.has(node.id)) continue
    included.add(node.id)
    for (const child of children.get(node.id) ?? []) pending.push(child)
  }
  const nodes = flow.nodes.filter((node) => included.has(node.id))
    .map((node) => node.id === nodeId ? { ...node, parentId: null } : node)
  const contentIds = new Set(nodes.flatMap((node) => node.contentIds))
  return { nodes, contents: flow.contents.filter((content) => contentIds.has(content.id)) }
}
