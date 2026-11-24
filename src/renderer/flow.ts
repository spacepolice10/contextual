import type { Flow, FlowNode } from "../diagrams"

const kindLabels: Record<FlowNode["kind"], string> = {
  root: "",
  goal: "Цель",
  task: "Задача",
  comment: "Комментарий",
  solution: "Решение",
  evidence: "Доказательство",
}

export interface FlowSelection {
  nodeId?: number
  contentIds: number[]
}

function escapeLabel(name: string) {
  return name.replace(/["#&<>%\\\n\r]/g, (character) => `#${character.charCodeAt(0)};`)
}

export function flowView({ nodes, contents }: Flow) {
  const lines = ["flowchart TD"]
  const selections = new Map<string, FlowSelection>()
  const byContent = new Map(contents.map((content) => [content.id, content]))
  const parents = new Set(nodes.map((node) => node.parentId))
  const grouped = new Map<number, FlowNode[]>()
  for (const node of nodes) {
    if (node.kind !== "task" || node.parentId === null || node.contentIds.length !== 1 || parents.has(node.id)) continue
    const siblings = grouped.get(node.parentId) ?? []
    siblings.push(node)
    grouped.set(node.parentId, siblings)
  }

  for (const node of nodes) {
    const siblings = node.parentId === null ? undefined : grouped.get(node.parentId)
    if (siblings && siblings.length > 1 && siblings.includes(node)) {
      if (siblings[0] !== node) continue
      const id = `tasks${node.parentId}`
      const names = siblings.map((task) => `• ${escapeLabel(task.name.length > 92 ? task.name.slice(0, 89) + "…" : task.name)}`)
      lines.push(`  ${id}["Задачи · ${siblings.length}<br/>${names.join("<br/>")}"]:::task`)
      lines.push(`  node${node.parentId} --> ${id}`)
      selections.set(id, { contentIds: siblings.flatMap((task) => task.contentIds) })
      continue
    }

    const { contentIds } = node
    const contents = contentIds.map((id) => byContent.get(id)).filter((content) => content !== undefined)
    const caption = contents.length > 1
      ? `${node.kind === "task" ? "Задачи" : "Записи"} · ${contents.length}<br/>${contents.map(({ name }) => `• ${escapeLabel(name)}`).join("<br/>")}`
      : node.kind === "root" ? escapeLabel(node.name) : `${kindLabels[node.kind]}<br/>${escapeLabel(node.name)}`
    const id = `node${node.id}`
    lines.push(`  ${id}["${caption}"]:::${node.kind}`)
    if (node.parentId !== null) lines.push(`  node${node.parentId} --> ${id}`)
    if (contentIds.length) selections.set(id, { nodeId: node.id, contentIds })
  }
  return { source: lines.join("\n"), selections }
}

export function overlayRootButton(svg: SVGSVGElement, root: FlowNode) {
  const node = svg.querySelector<SVGGElement>(`g.pele-node[data-id="node${root.id}"]`)
  if (!node) return

  const bounds = node.getBBox()
  const size = 28
  const overlay = document.createElementNS("http://www.w3.org/2000/svg", "foreignObject")
  overlay.setAttribute("x", String(bounds.x + bounds.width - size / 2))
  overlay.setAttribute("y", String(bounds.y - size / 2))
  overlay.setAttribute("width", String(size))
  overlay.setAttribute("height", String(size))

  const button = document.createElement("button")
  button.type = "button"
  button.className = "root-add-button"
  button.setAttribute("aria-label", "Добавить цель")
  button.setAttribute("aria-haspopup", "dialog")
  button.textContent = "+"
  overlay.append(button)
  node.append(overlay)
}
