import { mkdirSync } from "node:fs"
import { dirname } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { and, eq, isNull, max, ne } from "drizzle-orm"
import { goalTextLimit, type ContentKind, type ContentStatus, type Diagram, type FlowContent, type FlowNode, type WorkflowAction, type TaskPlanGroup, type ExecutionMode, type TaskContext } from "../diagrams"
import { migrate } from "./migrations"
import { agentsWork, contentVersions, contents, diagrams, nodeContents, nodes } from "./schema"
import { read, saveFlow, type LibraryDatabase } from "./sqlite"
import { flattenPlan, prerequisiteTasks, taskOrder } from "./workflow"

type ContentRow = typeof contents.$inferSelect
type NodeRow = typeof nodes.$inferSelect
type WorkRow = typeof agentsWork.$inferSelect
const now = () => new Date().toISOString()

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
  const [rows, placements, links, versions, work] = await Promise.all([
    db.select().from(contents).where(id === undefined ? undefined : inFlow(contents, id)).orderBy(contents.id),
    db.select().from(nodes).where(id === undefined ? undefined : inFlow(nodes, id)).orderBy(nodes.id),
    db.select().from(nodeContents).where(id === undefined ? undefined : eq(nodeContents.diagramId, id)).orderBy(nodeContents.position),
    db.select().from(contentVersions).where(id === undefined ? undefined : eq(contentVersions.diagramId, id)).orderBy(contentVersions.id),
    db.select().from(agentsWork).where(id === undefined ? undefined : eq(agentsWork.diagramId, id)).orderBy(agentsWork.id),
  ])
  return entries.map((entry) => {
    const flowNodes = placements.filter((node) => node.diagramId === entry.id)
    if (!flowNodes.length) return entry

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
    return { ...entry, flow: {
      contents: flowContents,
      nodes: flowNodes.map((node) => projectNode(node, byNode.get(node.id) ?? [])),
      versions: versions.filter((row) => row.diagramId === entry.id).map(({ diagramId, ...row }) => row),
      agentsWork: work.filter((row) => row.diagramId === entry.id).map(({ diagramId, ...row }) => row),
    } }
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
    db.exec("PRAGMA journal_mode=WAL")
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

async function insertVersion(db: LibraryDatabase, content: ContentRow, input: {
  name: string; body: string | null; parentVersionId: number | null; agentsWorkId?: number
}) {
  const [version] = await db.insert(contentVersions).values({
    ...input, diagramId: content.diagramId, contentId: content.id, createdAt: now(),
  }).returning()
  return version
}

async function getVersion(db: LibraryDatabase, diagramId: number, contentId: number, versionId: number) {
  const version = await db.select().from(contentVersions).where(and(
    eq(contentVersions.diagramId, diagramId), eq(contentVersions.contentId, contentId), eq(contentVersions.id, versionId),
  )).get()
  if (!version) throw new Error("Version not found for this result.")
  return version
}

async function insertContent(db: LibraryDatabase, diagramId: number, input: {
  parentId: number | null; kind: ContentKind; name: string; body?: string | null
  position?: number; executionStage?: number; agentsWorkId?: number
}) {
  const { agentsWorkId, position, executionStage, ...data } = input
  let order: { position?: number; executionStage?: number } = {}
  if (input.kind === "task") {
    if (input.parentId === null) throw new Error("A task needs a goal or task parent.")
    const parent = await getContent(db, diagramId, input.parentId)
    if (parent.kind !== "goal" && parent.kind !== "task") throw new Error("A task needs a goal or task parent.")
    if (parent.kind === "task" && parent.status !== "planned") throw new Error("Define subtasks before approving the parent task.")
    const siblings = await children(db, diagramId, input.parentId, "task")
    order = {
      position: position ?? (Math.max(-1, ...siblings.map((item) => item.position ?? item.id)) + 1),
      executionStage: executionStage ?? (Math.max(-1, ...siblings.map((item) => item.executionStage ?? item.id)) + 1),
    }
  }
  const [content] = await db.insert(contents).values({
    ...data, ...order, diagramId, id: await nextId(db, contents, diagramId), status: initialStatus[input.kind] ?? null,
  }).returning()
  if (content.kind !== "solution") return content
  const version = await insertVersion(db, content, {
    name: content.name, body: content.body, parentVersionId: null, agentsWorkId,
  })
  const [updated] = await db.update(contents).set({ latestVersionId: version.id })
    .where(inFlow(contents, diagramId, content.id)).returning()
  if (content.parentId !== null) await markTaskForReview(db, diagramId, content.parentId)
  return updated
}

async function markTaskForReview(db: LibraryDatabase, diagramId: number, taskId: number) {
  if ((await children(db, diagramId, taskId, "task")).length) return
  await db.update(contents).set({ status: "review" }).where(and(
    inFlow(contents, diagramId, taskId), eq(contents.kind, "task"), ne(contents.status, "accepted"),
  ))
}

async function appendVersion(db: LibraryDatabase, current: ContentRow, input: {
  name: string; body: string | null; parentVersionId: number; agentsWorkId?: number
}) {
  if (current.kind !== "solution") throw new Error("Only results have versions.")
  await getVersion(db, current.diagramId, current.id, input.parentVersionId)
  const version = await insertVersion(db, current, input)
  const [updated] = await db.update(contents).set({
    name: version.name, body: version.body, latestVersionId: version.id, status: "review",
  }).where(inFlow(contents, current.diagramId, current.id)).returning()
  if (current.parentId !== null) await markTaskForReview(db, current.diagramId, current.parentId)
  return { updated, version }
}

async function updateContentRecord(db: LibraryDatabase, current: ContentRow, changes: { name?: string; body?: string }) {
  const name = changes.name ?? current.name
  const body = changes.body ?? current.body
  if (name === current.name && body === current.body) return current
  if (current.kind === "solution") {
    if (current.latestVersionId === null) throw new Error("Result has no base version.")
    return (await appendVersion(db, current, { name, body, parentVersionId: current.latestVersionId })).updated
  }
  const [updated] = await db.update(contents).set({ name, body })
    .where(inFlow(contents, current.diagramId, current.id)).returning()
  return updated
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

async function contentPlacement(db: LibraryDatabase, task: ContentRow) {
  const placements = await db.select({ id: nodeContents.nodeId }).from(nodeContents)
    .where(and(eq(nodeContents.diagramId, task.diagramId), eq(nodeContents.contentId, task.id)))
    .orderBy(nodeContents.nodeId).all()
  for (const { id } of placements) {
    if ((await references(db, task.diagramId, id)).length === 1) return id
  }
  if (placements.length) return (await insertPlacement(db, task.diagramId, placements[0].id, [task])).id
}

async function addBranch(db: LibraryDatabase, diagramId: number, parentNodeId: number, kind: ContentKind, name: string,
  order?: { position: number; executionStage: number }) {
  const parent = await getNode(db, diagramId, parentNodeId)
  const underRoot = parent.parentId === null
  if ((kind === "goal") !== underRoot) throw new Error("Goals go directly under the root. Other nodes belong beneath a goal.")
  const parentContent = underRoot ? null : await placedContent(db, diagramId, parentNodeId)
  const content = await insertContent(db, diagramId, { kind, name, parentId: parentContent?.id ?? null, ...order })
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
    .orderBy(contents.position, contents.id).all()
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

export function beginPlanning(path: string, diagramId: number, goalNodeId: number, context: object) {
  return saveFlow(path, async (db) => {
    const goal = await placedContent(db, diagramId, goalNodeId)
    if (goal.kind !== "goal") throw new Error("Choose a goal.")
    const [work] = await db.insert(agentsWork).values({
      diagramId, contentId: goal.id, kind: "planning", status: "running", agent: "codex",
      input: JSON.stringify(context), createdAt: now(), startedAt: now(),
    }).returning()
    return work.id
  })
}

export function saveGoalTasks(path: string, diagramId: number, goalId: number, groups: TaskPlanGroup[], workId?: number) {
  return saveFlow(path, async (db) => {
    const goal = await placedContent(db, diagramId, goalId)
    if (goal.kind !== "goal") throw new Error("Цель не найдена.")
    if (workId !== undefined) {
      const work = await getWork(db, workId)
      if (work.diagramId !== diagramId || work.contentId !== goal.id || work.kind !== "planning" || work.status !== "running") {
        throw new Error("Planning work is no longer active.")
      }
    }
    const saved = []
    if (!(await children(db, diagramId, goal.id, "task")).length) {
      for (const { name, ...order } of flattenPlan(groups)) saved.push(await addBranch(db, diagramId, goalId, "task", name, order))
    }
    if (workId !== undefined) await finishWork(db, workId, "completed")
    return saved
  })
}

// External editors can arrange a draft plan without changing its visual nodes.
export function configureTaskPlan(path: string, diagramId: number, parentId: number,
  groups: { mode: ExecutionMode; taskIds: number[] }[]) {
  return saveFlow(path, async (db) => {
    const parent = await getContent(db, diagramId, parentId)
    if (parent.kind !== "goal" && parent.kind !== "task") throw new Error("Choose a plan parent.")
    const tasks = await children(db, diagramId, parentId, "task")
    const ordered = flattenPlan(groups.map(({ mode, taskIds }) => ({ mode, tasks: taskIds.map(String) })))
    const ids = new Set(ordered.map(({ name }) => Number(name)))
    if (!tasks.length || tasks.length !== ordered.length || ids.size !== ordered.length ||
        tasks.some((task) => !ids.has(task.id) || task.status !== "planned")) {
      throw new Error("Provide every draft task under this parent exactly once; approved plans cannot be reordered.")
    }
    for (const { name, ...order } of ordered) {
      await db.update(contents).set(order).where(inFlow(contents, diagramId, Number(name)))
    }
    return children(db, diagramId, parentId, "task")
  })
}

export function createFlow(path: string, name: string) {
  return saveFlow(path, async (db) => {
    const [{ id }] = await db.insert(diagrams).values({ name, source: "" }).returning({ id: diagrams.id })
    const root = await insertPlacement(db, id, null, [], name)
    return { id, name, source: "", flow: { nodes: [root], contents: [], versions: [], agentsWork: [] } }
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
      linked[0] = await updateContentRecord(db, linked[0], { name: changes.name })
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
  return saveFlow(path, async (db) => updateContentRecord(db, await getContent(db, diagramId, contentId), changes))
}

export function createResultVersion(path: string, diagramId: number, contentId: number, parentVersionId: number,
  changes: { name?: string; body?: string }) {
  return saveFlow(path, async (db) => {
    const current = await getContent(db, diagramId, contentId)
    const base = await getVersion(db, diagramId, contentId, parentVersionId)
    return (await appendVersion(db, current, {
      name: changes.name ?? base.name, body: changes.body ?? base.body, parentVersionId,
    })).version
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

async function getWork(db: LibraryDatabase, id: number) {
  const work = await db.select().from(agentsWork).where(eq(agentsWork.id, id)).get()
  if (!work) throw new Error("AgentsWork not found.")
  return work
}

async function finishWork(db: LibraryDatabase, id: number, status: "completed" | "failed" | "interrupted" | "cancelled",
  error?: string, resultContentId?: number) {
  await db.update(agentsWork).set({ status, error, resultContentId, finishedAt: now() })
    .where(and(eq(agentsWork.id, id), eq(agentsWork.status, "running")))
}

async function queueWork(db: LibraryDatabase, diagramId: number, contentId: number,
  kind: "execution" | "revision", options: {
    retryOfId?: number; baseVersionId?: number | null; resultContentId?: number | null; instruction?: string | null
  } = {}) {
  const [work] = await db.insert(agentsWork).values({
    ...options, diagramId, contentId, kind, status: "queued", agent: "codex", createdAt: now(),
  }).returning()
  if (kind === "execution") {
    await db.update(contents).set({ status: "queued" }).where(inFlow(contents, diagramId, contentId))
  }
  return work
}

async function queuePlan(db: LibraryDatabase, diagramId: number, parentId: number) {
  for (const task of await children(db, diagramId, parentId, "task")) {
    const nested = await children(db, diagramId, task.id, "task")
    if (nested.length) {
      if (task.status === "planned") {
        await db.update(contents).set({ status: "queued" }).where(inFlow(contents, diagramId, task.id))
      }
      if (task.status === "planned" || task.status === "queued") {
        await queuePlan(db, diagramId, task.id)
        if ((await children(db, diagramId, task.id, "task")).every((child) => child.status === "accepted")) {
          await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, diagramId, task.id))
        }
      }
    } else if (task.status === "planned") {
      await queueWork(db, diagramId, task.id, "execution")
    }
  }
}

async function completeParentGroups(db: LibraryDatabase, diagramId: number, task: ContentRow) {
  let parentId = task.parentId
  while (parentId !== null) {
    const parent = await getContent(db, diagramId, parentId)
    if (parent.kind !== "task" || parent.status !== "queued") return
    const tasks = await children(db, diagramId, parent.id, "task")
    if (!tasks.length || tasks.some(({ status }) => status !== "accepted")) return
    await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, diagramId, parent.id))
    parentId = parent.parentId
  }
}

async function selectResultVersion(db: LibraryDatabase, result: ContentRow, versionId: number) {
  if (result.kind !== "solution") throw new Error("Choose a result.")
  await getVersion(db, result.diagramId, result.id, versionId)
  await db.update(contents).set({
    selectedVersionId: versionId, status: result.latestVersionId === versionId ? "accepted" : "review",
  }).where(inFlow(contents, result.diagramId, result.id))
  if (result.parentId === null) return
  const task = await getContent(db, result.diagramId, result.parentId)
  if (task.kind !== "task") return
  const nested = await children(db, result.diagramId, task.id, "task")
  if (nested.some(({ status }) => status !== "accepted")) return
  await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, result.diagramId, task.id))
  await completeParentGroups(db, result.diagramId, task)
}

export function acceptResultVersion(path: string, diagramId: number, contentId: number, versionId: number) {
  return saveFlow(path, async (db) => selectResultVersion(db, await getContent(db, diagramId, contentId), versionId))
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
      } else {
        let ancestor = content
        while (ancestor.kind === "task" && ancestor.parentId !== null) ancestor = await getContent(db, diagramId, ancestor.parentId)
        if (ancestor.kind !== "goal" || ancestor.status !== "approved") throw new Error("Сначала примите план цели.")
        if (content.status !== "planned" && content.status !== "queued") throw new Error("This task is not a draft group.")
        await db.update(contents).set({ status: "queued" }).where(inFlow(contents, diagramId, content.id))
      }
      await queuePlan(db, diagramId, content.id)
      if (content.kind === "task" && (await children(db, diagramId, content.id, "task")).every(({ status }) => status === "accepted")) {
        await db.update(contents).set({ status: "accepted" }).where(inFlow(contents, diagramId, content.id))
        await completeParentGroups(db, diagramId, content)
      }
    } else if (action === "retry_task") {
      if (content.kind !== "task" || content.status !== "failed") throw new Error("Повтор доступен для задачи с ошибкой.")
      const previous = (await db.select().from(agentsWork).where(and(
        eq(agentsWork.diagramId, diagramId), eq(agentsWork.contentId, content.id), eq(agentsWork.kind, "execution"),
        eq(agentsWork.status, "failed"),
      )).orderBy(agentsWork.id)).at(-1)
      await queueWork(db, diagramId, content.id, "execution", { retryOfId: previous?.id })
    } else {
      if (content.kind !== "solution" || content.latestVersionId === null) throw new Error("Выберите результат.")
      if (content.selectedVersionId === content.latestVersionId) return
      await selectResultVersion(db, content, content.latestVersionId)
    }
  })
}

export function queueRevision(path: string, diagramId: number, resultId: number, baseVersionId: number, instruction: string) {
  return saveFlow(path, async (db) => {
    const result = await getContent(db, diagramId, resultId)
    if (result.kind !== "solution" || result.parentId === null) throw new Error("Choose a task result.")
    const task = await getContent(db, diagramId, result.parentId)
    if (task.kind !== "task") throw new Error("Choose a task result.")
    await getVersion(db, diagramId, resultId, baseVersionId)
    if (!instruction.trim()) throw new Error("Describe the revision.")
    return queueWork(db, diagramId, task.id, "revision", { baseVersionId, resultContentId: resultId, instruction })
  })
}

export function retryRevision(path: string, diagramId: number, workId: number) {
  return saveFlow(path, async (db) => {
    const work = await getWork(db, workId)
    if (work.diagramId !== diagramId || work.kind !== "revision" || work.status !== "failed") {
      throw new Error("Choose a failed revision.")
    }
    return queueWork(db, diagramId, work.contentId, "revision", {
      retryOfId: work.id, baseVersionId: work.baseVersionId, resultContentId: work.resultContentId, instruction: work.instruction,
    })
  })
}

export interface TaskResult {
  solution: { name: string; body: string }
  comments: { name: string; body?: string }[]
  evidence: { name: string; body?: string }[]
}

export function recoverInterruptedTasks(path: string) {
  return saveFlow(path, async (db) => {
    const interrupted = await db.select().from(agentsWork).where(eq(agentsWork.status, "running")).orderBy(agentsWork.id)
    for (const work of interrupted) {
      await finishWork(db, work.id, "interrupted", "Application closed before work finished.")
      // Planning is explicitly retried through the existing goal UI.
      if (work.kind === "planning") continue
      const task = await getContent(db, work.diagramId, work.contentId)
      if (work.kind === "execution" && task.status !== "running") continue
      await queueWork(db, work.diagramId, work.contentId, work.kind, {
        retryOfId: work.id, baseVersionId: work.baseVersionId, resultContentId: work.resultContentId, instruction: work.instruction,
      })
    }
  })
}

export function claimTask(path: string) {
  return saveFlow(path, async (db) => {
    const queued = await db.select().from(agentsWork).where(eq(agentsWork.status, "queued")).orderBy(agentsWork.id)
    const byFlow = new Map<number, ContentRow[]>()
    for (const work of queued) {
      if (work.kind === "planning") continue
      let all = byFlow.get(work.diagramId)
      if (!all) {
        all = await db.select().from(contents).where(inFlow(contents, work.diagramId)).orderBy(contents.id)
        byFlow.set(work.diagramId, all)
      }
      const task = all.find(({ id }) => id === work.contentId)
      if (!task || task.kind !== "task") continue
      if (work.kind === "execution" && (task.status !== "queued" ||
          all.some((item) => item.kind === "task" && item.parentId === task.id))) {
        await db.update(agentsWork).set({ status: "cancelled", finishedAt: now(), error: "Task is no longer executable." })
          .where(eq(agentsWork.id, work.id))
        continue
      }
      const dependencies = prerequisiteTasks(all, task)
      if (work.kind === "execution" && dependencies === null) continue
      const flow = await db.select().from(diagrams).where(eq(diagrams.id, task.diagramId)).get()
      const immediateParent = all.find(({ id }) => id === task.parentId)
      let parent = immediateParent
      while (parent && parent.kind !== "goal" && parent.parentId !== null) parent = all.find(({ id }) => id === parent!.parentId)
      const goal = parent?.kind === "goal" ? parent : null
      const siblings = all.filter((item) => item.kind === "task" && item.parentId === task.parentId).sort(taskOrder)
      const prerequisiteIds = new Set((dependencies ?? []).map(({ id }) => id))
      const prerequisites: TaskContext["prerequisites"] = []
      for (const result of all.filter((item) => item.kind === "solution" && item.parentId !== null &&
          prerequisiteIds.has(item.parentId) && item.selectedVersionId !== null)) {
        const version = await getVersion(db, task.diagramId, result.id, result.selectedVersionId!)
        prerequisites.push({
          taskId: result.parentId!, task: all.find(({ id }) => id === result.parentId)!.name,
          resultId: result.id, versionId: version.id, name: version.name, body: version.body,
        })
      }
      const context: TaskContext = {
        flow: flow!.name, goal: goal?.name, goalDetails: goal?.body, task: task.name, details: task.body,
        parentTask: immediateParent?.kind === "task" ? immediateParent.name : undefined,
        siblingTasks: siblings.map(({ name }) => name), prerequisites,
      }
      if (work.kind === "revision") {
        if (work.resultContentId === null || work.baseVersionId === null || work.instruction === null) {
          throw new Error("Revision is missing its base version or instruction.")
        }
        const base = await getVersion(db, task.diagramId, work.resultContentId, work.baseVersionId)
        context.revision = {
          resultId: work.resultContentId, versionId: base.id, name: base.name, body: base.body, instruction: work.instruction,
        }
      }
      await db.update(agentsWork).set({ status: "running", startedAt: now(), input: JSON.stringify(context) })
        .where(eq(agentsWork.id, work.id))
      if (work.kind === "execution") {
        await db.update(contents).set({ status: "running" }).where(inFlow(contents, task.diagramId, task.id))
      }
      return { id: work.id, context }
    }
  })
}

export function failWork(path: string, workId: number, error: string) {
  return saveFlow(path, async (db) => {
    const work = await getWork(db, workId)
    if (work.status !== "running") return
    await finishWork(db, workId, "failed", error)
    if (work.kind === "execution") {
      await db.update(contents).set({ status: "failed" }).where(and(
        inFlow(contents, work.diagramId, work.contentId), eq(contents.status, "running"),
      ))
    }
  })
}

export function saveTaskResult(path: string, workId: number, result: TaskResult) {
  return saveFlow(path, async (db) => {
    const work = await getWork(db, workId)
    if (work.status !== "running" || work.kind === "planning") return
    const { diagramId, contentId: taskId } = work
    const task = await getContent(db, diagramId, taskId)
    if (task.kind !== "task") throw new Error("AgentsWork target is not a task.")
    const taskNodeId = await contentPlacement(db, task)
    let solution: ContentRow
    let solutionNodeId: number | undefined
    if (work.kind === "revision") {
      if (work.resultContentId === null || work.baseVersionId === null) throw new Error("Revision has no base version.")
      const current = await getContent(db, diagramId, work.resultContentId)
      solution = (await appendVersion(db, current, {
        ...result.solution, parentVersionId: work.baseVersionId, agentsWorkId: work.id,
      })).updated
      solutionNodeId = await contentPlacement(db, solution)
    } else {
      solution = await insertContent(db, diagramId, {
        ...result.solution, kind: "solution", parentId: taskId, agentsWorkId: work.id,
      })
      if (taskNodeId !== undefined) solutionNodeId = (await insertPlacement(db, diagramId, taskNodeId, [solution])).id
    }
    for (const item of result.comments) {
      const content = await insertContent(db, diagramId, { ...item, kind: "comment", parentId: taskId })
      if (taskNodeId !== undefined) await insertPlacement(db, diagramId, taskNodeId, [content])
    }
    for (const item of result.evidence) {
      const content = await insertContent(db, diagramId, { ...item, kind: "evidence", parentId: solution.id })
      if (solutionNodeId !== undefined) await insertPlacement(db, diagramId, solutionNodeId, [content])
    }
    await finishWork(db, workId, "completed", undefined, solution.id)
  })
}
