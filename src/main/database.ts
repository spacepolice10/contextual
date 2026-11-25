import { mkdirSync } from "node:fs"
import { dirname } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { and, eq, isNull, max } from "drizzle-orm"
import { goalTextLimit, type ContentKind, type ContentStatus, type Diagram, type FlowContent, type FlowNode, type WorkflowAction } from "../diagrams"
import { migrate } from "./migrations"
import { contents, diagrams, nodeContents, nodes } from "./schema"
import { read, saveFlow, type LibraryDatabase } from "./sqlite"

type ContentRow = typeof contents.$inferSelect
type NodeRow = typeof nodes.$inferSelect

const initialStatus: Partial<Record<ContentKind, ContentStatus>> = {
  goal: "draft",
  task: "planned",
  solution: "review",
}

function inFlow(table: typeof contents | typeof nodes, diagramId: number, id?: number) {
  return and(eq(table.diagramId, diagramId), id === undefined ? undefined : eq(table.id, id))
}

function projectNode(node: NodeRow, refs: FlowContent[]): FlowNode {
  return {
    id: node.id,
    parentId: node.parentId,
    kind: refs[0]?.kind ?? "root",
    name: node.name ?? refs[0]?.name ?? "",
    contentIds: refs.map((content) => content.id),
  }
}

async function readDiagrams(db: LibraryDatabase, id?: number): Promise<Diagram[]> {
  const entries = await db.select().from(diagrams).where(id === undefined ? undefined : eq(diagrams.id, id)).orderBy(diagrams.id)
  const [rows, placements, links] = await Promise.all([
    db.select().from(contents).where(id === undefined ? undefined : inFlow(contents, id)).orderBy(contents.id),
    db.select().from(nodes).where(id === undefined ? undefined : inFlow(nodes, id)).orderBy(nodes.id),
    db.select().from(nodeContents).where(id === undefined ? undefined : eq(nodeContents.diagramId, id)).orderBy(nodeContents.position),
  ])
  return entries.map((entry) => {
    const flowNodes = placements.filter((node) => node.diagramId === entry.id)

    const flowContents = rows.filter((content) => content.diagramId === entry.id)
      .map(({ diagramId, ...content }) => content)
    const byId = new Map(flowContents.map((content) => [content.id, content]))
    const byNode = new Map<number, FlowContent[]>()
    for (const link of links) {
      if (link.diagramId !== entry.id) continue
      const content = byId.get(link.contentId)
      if (!content) continue
      const refs = byNode.get(link.nodeId) ?? []
      refs.push(content)
      byNode.set(link.nodeId, refs)
    }
    return { ...entry, flow: { contents: flowContents, nodes: flowNodes.map((node) => projectNode(node, byNode.get(node.id) ?? [])) } }
  })
}

export function readLibrary(path: string, id?: number) {
  return read(path, (db) => readDiagrams(db, id))
}

export function initializeLibrary(path: string) {
  mkdirSync(dirname(path), { recursive: true })
  const db = new DatabaseSync(path, { timeout: 5000 })
  try {
    migrate(db)
  } finally {
    db.close()
  }
}

async function getContent(db: LibraryDatabase, diagramId: number, id: number): Promise<ContentRow> {
  const content = await db.select().from(contents).where(inFlow(contents, diagramId, id)).get()
  if (!content) throw new Error("Content not found in this flow.")
  return content
}

async function getNode(db: LibraryDatabase, diagramId: number, id: number) {
  const node = await db.select().from(nodes).where(inFlow(nodes, diagramId, id)).get()
  if (!node) throw new Error("Node not found in this flow.")
  return node
}

async function getRoot(db: LibraryDatabase, diagramId: number) {
  const root = await db.select().from(nodes)
    .where(and(inFlow(nodes, diagramId), isNull(nodes.parentId))).get()
  if (!root) throw new Error("Поток не найден.")
  return root
}

async function references(db: LibraryDatabase, diagramId: number, nodeId: number) {
  return db.select({ contentId: nodeContents.contentId }).from(nodeContents)
    .where(and(eq(nodeContents.diagramId, diagramId), eq(nodeContents.nodeId, nodeId)))
    .orderBy(nodeContents.position)
}

async function placedContent(db: LibraryDatabase, diagramId: number, nodeId: number) {
  const links = await references(db, diagramId, nodeId)
  if (links.length !== 1) throw new Error("Choose an individual content record.")
  return getContent(db, diagramId, links[0].contentId)
}

async function nextId(db: LibraryDatabase, table: typeof contents | typeof nodes, diagramId: number) {
  const row = await db.select({ id: max(table.id) }).from(table).where(inFlow(table, diagramId)).get()
  return (row?.id ?? 0) + 1
}

async function insertContent(db: LibraryDatabase, diagramId: number, input: {
  parentId: number | null; kind: ContentKind; name: string; body?: string | null
}) {
  const [content] = await db.insert(contents).values({
    ...input, diagramId, id: await nextId(db, contents, diagramId), status: initialStatus[input.kind] ?? null,
  }).returning()
  if (content.kind === "solution" && content.parentId !== null) {
    await markTaskForReview(db, diagramId, content.parentId)
  }
  return content
}

async function markTaskForReview(db: LibraryDatabase, diagramId: number, taskId: number) {
  await db.update(contents).set({ status: "review" }).where(and(
    inFlow(contents, diagramId, taskId), eq(contents.kind, "task"),
  ))
}

async function insertPlacement(db: LibraryDatabase, diagramId: number, parentId: number | null, refs: ContentRow[], name?: string) {
  const [node] = await db.insert(nodes).values({ diagramId, parentId, id: await nextId(db, nodes, diagramId), name }).returning()
  if (refs.length) {
    await db.insert(nodeContents).values(refs.map((content, position) => ({
      diagramId, nodeId: node.id, contentId: content.id, position,
    })))
  }
  return projectNode(node, refs)
}

async function taskPlacement(db: LibraryDatabase, task: ContentRow) {
  const placements = await db.select({ id: nodeContents.nodeId }).from(nodeContents)
    .where(and(eq(nodeContents.diagramId, task.diagramId), eq(nodeContents.contentId, task.id)))
    .orderBy(nodeContents.nodeId).all()
  for (const { id } of placements) {
    if ((await references(db, task.diagramId, id)).length === 1) return id
  }
  if (placements.length) return (await insertPlacement(db, task.diagramId, placements[0].id, [task])).id
}

async function addBranch(db: LibraryDatabase, diagramId: number, parentNodeId: number, kind: ContentKind, name: string) {
  const parent = await getNode(db, diagramId, parentNodeId)
  const underRoot = parent.parentId === null
  if ((kind === "goal") !== underRoot) throw new Error("Goals go directly under the root. Other nodes belong beneath a goal.")
  const parentContent = underRoot ? null : await placedContent(db, diagramId, parentNodeId)
  const content = await insertContent(db, diagramId, { kind, name, parentId: parentContent?.id ?? null })
  return insertPlacement(db, diagramId, parentNodeId, [content])
}

export function createGoal(path: string, diagramId: number, text: string) {
  if (!Number.isSafeInteger(diagramId) || typeof text !== "string" || !text.trim() || text.length > goalTextLimit) {
    throw new Error("Введите цель длиной до 2000 символов.")
  }
  return saveFlow(path, async (db) => addBranch(db, diagramId, (await getRoot(db, diagramId)).id, "goal", text.trim()))
}

function children(db: LibraryDatabase, diagramId: number, parent: number, kind: ContentKind) {
  return db.select().from(contents)
    .where(and(inFlow(contents, diagramId), eq(contents.parentId, parent), eq(contents.kind, kind)))
    .orderBy(contents.id).all()
}

export function readGoal(path: string, diagramId: number, goalId: number) {
  if (!Number.isSafeInteger(diagramId) || !Number.isSafeInteger(goalId)) throw new Error("Цель не найдена.")
  return read(path, async (db) => {
    const goal = await placedContent(db, diagramId, goalId)
    if (goal.kind !== "goal") throw new Error("Цель не найдена.")
    const flow = await db.select().from(diagrams).where(eq(diagrams.id, diagramId)).get()
    return { context: { flow: flow!.name, goal: goal.name, details: goal.body }, hasTasks: (await children(db, diagramId, goal.id, "task")).length > 0 }
  })
}

export function saveGoalTasks(path: string, diagramId: number, goalId: number, names: string[]) {
  return saveFlow(path, async (db) => {
    const goal = await placedContent(db, diagramId, goalId)
    if (goal.kind !== "goal") throw new Error("Цель не найдена.")
    if ((await children(db, diagramId, goal.id, "task")).length) return []
    const saved = []
    for (const name of names) saved.push(await addBranch(db, diagramId, goalId, "task", name))
    return saved
  })
}

export function createFlow(path: string, name: string) {
  return saveFlow(path, async (db) => {
    const [{ id }] = await db.insert(diagrams).values({ name }).returning({ id: diagrams.id })
    const root = await insertPlacement(db, id, null, [], name)
    return { id, name, flow: { nodes: [root], contents: [] } }
  })
}

export function renameFlow(path: string, diagramId: number, name: string) {
  return saveFlow(path, async (db) => {
    const root = await getRoot(db, diagramId)
    await db.update(nodes).set({ name }).where(inFlow(nodes, diagramId, root.id))
    await db.update(diagrams).set({ name }).where(eq(diagrams.id, diagramId))
    return (await readDiagrams(db, diagramId))[0]
  })
}

export function createFlowNode(path: string, diagramId: number, parentId: number, kind: ContentKind, name: string) {
  return saveFlow(path, (db) => addBranch(db, diagramId, parentId, kind, name))
}

export function updateFlowNode(path: string, diagramId: number, nodeId: number, changes: { name?: string; parentId?: number }) {
  return saveFlow(path, async (db) => {
    const node = await getNode(db, diagramId, nodeId)
    if (node.parentId === null) throw new Error("Use rename_flow to rename the root.")
    const refs = await references(db, diagramId, nodeId)
    const linked = await Promise.all(refs.map(({ contentId }) => getContent(db, diagramId, contentId)))
    const parentId = changes.parentId ?? node.parentId
    if (parentId !== node.parentId) {
      let parent = await getNode(db, diagramId, parentId)
      if ((parent.parentId === null) !== (linked[0]?.kind === "goal")) throw new Error("Choose a parent within the corresponding branch.")
      while (true) {
        if (parent.id === nodeId) throw new Error("A node cannot be moved beneath itself or its descendants.")
        if (parent.parentId === null) break
        parent = await getNode(db, diagramId, parent.parentId)
      }
      await db.update(nodes).set({ parentId }).where(inFlow(nodes, diagramId, nodeId))
    }

    if (changes.name !== undefined) {
      if (linked.length !== 1) throw new Error("Rename the individual content with update_content.")
      await db.update(contents).set({ name: changes.name }).where(inFlow(contents, diagramId, linked[0].id))
      linked[0].name = changes.name
    }
    return projectNode({ ...node, parentId }, linked)
  })
}

export function createContent(path: string, diagramId: number, input: {
  kind: ContentKind; name: string; body?: string; parentId?: number
}) {
  return saveFlow(path, async (db) => {
    await getRoot(db, diagramId)
    const parent = input.parentId === undefined ? null : await getContent(db, diagramId, input.parentId)
    if (input.kind === "goal" && parent) throw new Error("Goals cannot have a content parent.")
    return insertContent(db, diagramId, { ...input, parentId: parent?.id ?? null })
  })
}

export function updateContent(path: string, diagramId: number, contentId: number, changes: { name?: string; body?: string }) {
  return saveFlow(path, async (db) => {
    const current = await getContent(db, diagramId, contentId)
    const revised = current.kind === "solution" && current.status === "accepted"
      && changes.body !== undefined && changes.body !== current.body
    const [updated] = await db.update(contents).set({ ...changes, status: revised ? "review" : undefined })
      .where(inFlow(contents, diagramId, contentId)).returning()
    if (revised && current.parentId !== null) {
      const solutions = await children(db, diagramId, current.parentId, "solution")
      if (!solutions.some(({ status }) => status === "accepted")) await markTaskForReview(db, diagramId, current.parentId)
    }
    return updated
  })
}

export function placeContent(path: string, diagramId: number, parentId: number, contentIds: number[]) {
  return saveFlow(path, async (db) => {
    const parent = await getNode(db, diagramId, parentId)
    const refs = await Promise.all(contentIds.map((id) => getContent(db, diagramId, id)))
    if (refs.some((content) => (content.kind === "goal") !== (parent.parentId === null))) {
      throw new Error("Goals go directly under the root. Place other content beneath a goal.")
    }
    return insertPlacement(db, diagramId, parentId, refs)
  })
}

export function applyWorkflow(path: string, diagramId: number, action: WorkflowAction, contentId: number) {
  return saveFlow(path, async (db) => {
    const content = await getContent(db, diagramId, contentId)
    if (action === "approve_plan") {
      if (content.kind !== "goal" && content.kind !== "task") throw new Error("Выберите цель или задачу с подзадачами.")
      const tasks = await children(db, diagramId, content.id, "task")
      if (!tasks.length) throw new Error("Здесь пока нет задач.")
      if (content.kind === "goal") {
        await db.update(contents).set({ status: "approved" }).where(inFlow(contents, diagramId, content.id))
      }
      await db.update(contents).set({ status: "queued" }).where(and(
        inFlow(contents, diagramId), eq(contents.parentId, content.id), eq(contents.kind, "task"), eq(contents.status, "planned"),
      ))
    } else if (action === "retry_task") {
      if (content.kind !== "task" || content.status !== "failed") throw new Error("Повтор доступен для задачи с ошибкой.")
      await db.update(contents).set({ status: "queued" }).where(inFlow(contents, diagramId, content.id))
    } else {
      if (content.kind !== "solution") throw new Error("Выберите результат.")
      if (content.status === "accepted") return
      await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, diagramId, content.id))
      if (content.parentId !== null) {
        const parent = await getContent(db, diagramId, content.parentId)
        if (parent.kind === "task" && parent.status === "review") {
          await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, diagramId, parent.id))
        }
      }
    }
  })
}

export interface TaskResult {
  solution: { name: string; body: string }
  comments: { name: string; body?: string }[]
  evidence: { name: string; body?: string }[]
}

export function recoverInterruptedTasks(path: string) {
  return saveFlow(path, async (db) => {
    await db.update(contents).set({ status: "queued" }).where(and(eq(contents.kind, "task"), eq(contents.status, "running")))
  })
}

export async function claimTask(path: string) {
  const queued = await read(path, (db) => db.select().from(contents)
    .where(and(eq(contents.kind, "task"), eq(contents.status, "queued")))
    .orderBy(contents.id, contents.diagramId).get())
  if (!queued) return
  return saveFlow(path, async (db) => {
    const task = await getContent(db, queued.diagramId, queued.id)
    if (task.status !== "queued") return
    const flow = await db.select().from(diagrams).where(eq(diagrams.id, task.diagramId)).get()
    const immediateParent = task.parentId === null ? null : await getContent(db, task.diagramId, task.parentId)
    let parent = immediateParent
    while (parent && parent.kind !== "goal" && parent.parentId !== null) {
      parent = await getContent(db, task.diagramId, parent.parentId)
    }
    const goal = parent?.kind === "goal" ? parent : null
    const siblings = immediateParent ? await children(db, task.diagramId, immediateParent.id, "task") : []
    await db.update(contents).set({ status: "running" }).where(inFlow(contents, task.diagramId, task.id))
    return {
      diagramId: task.diagramId,
      id: task.id,
      context: {
        flow: flow!.name, goal: goal?.name, goalDetails: goal?.body, task: task.name, details: task.body,
        parentTask: immediateParent?.kind === "task" ? immediateParent.name : undefined,
        siblingTasks: siblings.map(({ name }) => name),
      },
    }
  })
}

export function failTask(path: string, diagramId: number, taskId: number) {
  return saveFlow(path, async (db) => {
    await db.update(contents).set({ status: "failed" }).where(and(inFlow(contents, diagramId, taskId), eq(contents.status, "running")))
  })
}

export function saveTaskResult(path: string, diagramId: number, taskId: number, result: TaskResult) {
  return saveFlow(path, async (db) => {
    const task = await getContent(db, diagramId, taskId)
    if (task.kind !== "task" || task.status !== "running") return
    const taskNodeId = await taskPlacement(db, task)
    const solution = await insertContent(db, diagramId, { ...result.solution, kind: "solution", parentId: taskId })
    const solutionNode = taskNodeId === undefined ? undefined : await insertPlacement(db, diagramId, taskNodeId, [solution])
    for (const item of result.comments) {
      const content = await insertContent(db, diagramId, { ...item, kind: "comment", parentId: taskId })
      if (taskNodeId !== undefined) await insertPlacement(db, diagramId, taskNodeId, [content])
    }
    for (const item of result.evidence) {
      const content = await insertContent(db, diagramId, { ...item, kind: "evidence", parentId: solution.id })
      if (solutionNode) await insertPlacement(db, diagramId, solutionNode.id, [content])
    }
  })
}
