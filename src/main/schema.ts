import { integer, primaryKey, sqliteTable, text } from "drizzle-orm/sqlite-core"
import type { AgentsWorkKind, AgentsWorkStatus, ChangeEvent, ContentKind, ContentStatus } from "../diagrams"

export const diagrams = sqliteTable("diagrams", {
  id: integer().primaryKey(),
  name: text().notNull(),
  source: text().notNull(),
})

export const contents = sqliteTable("flow_contents", {
  diagramId: integer("diagram_id").notNull(),
  id: integer().notNull(),
  parentId: integer("parent_id"),
  kind: text().$type<ContentKind>().notNull(),
  name: text().notNull(),
  body: text(),
  status: text().$type<ContentStatus>(),
  position: integer(),
  executionStage: integer("execution_stage"),
  latestVersionId: integer("latest_version_id"),
  selectedVersionId: integer("selected_version_id"),
}, (table) => [primaryKey({ columns: [table.diagramId, table.id] })])

export const nodes = sqliteTable("flow_nodes", {
  diagramId: integer("diagram_id").notNull(),
  id: integer().notNull(),
  parentId: integer("parent_id"),
  name: text(),
}, (table) => [primaryKey({ columns: [table.diagramId, table.id] })])

export const nodeContents = sqliteTable("flow_node_contents", {
  diagramId: integer("diagram_id").notNull(),
  nodeId: integer("node_id").notNull(),
  contentId: integer("content_id").notNull(),
  position: integer().notNull(),
}, (table) => [primaryKey({ columns: [table.diagramId, table.nodeId, table.contentId] })])

export const agentsWork = sqliteTable("agents_work", {
  id: integer().primaryKey({ autoIncrement: true }),
  diagramId: integer("diagram_id").notNull(),
  contentId: integer("content_id").notNull(),
  kind: text().$type<AgentsWorkKind>().notNull(),
  status: text().$type<AgentsWorkStatus>().notNull(),
  agent: text().notNull(),
  input: text(),
  instruction: text(),
  error: text(),
  retryOfId: integer("retry_of_id"),
  baseVersionId: integer("base_version_id"),
  resultContentId: integer("result_content_id"),
  createdAt: text("created_at").notNull(),
  startedAt: text("started_at"),
  finishedAt: text("finished_at"),
})

export const contentVersions = sqliteTable("content_versions", {
  id: integer().primaryKey({ autoIncrement: true }),
  diagramId: integer("diagram_id").notNull(),
  contentId: integer("content_id").notNull(),
  parentVersionId: integer("parent_version_id"),
  agentsWorkId: integer("agents_work_id"),
  name: text().notNull(),
  body: text(),
  createdAt: text("created_at").notNull(),
})

export const changeEvents = sqliteTable("change_events", {
  id: integer().primaryKey({ autoIncrement: true }),
  flowId: integer("diagram_id").notNull(),
  entity: text().$type<ChangeEvent["entity"]>().notNull(),
  entityId: integer("entity_id").notNull(),
  operation: text().$type<ChangeEvent["operation"]>().notNull(),
  createdAt: text("created_at").notNull(),
})
