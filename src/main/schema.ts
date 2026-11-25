import { integer, primaryKey, sqliteTable, text } from "drizzle-orm/sqlite-core"
import type { ContentKind, ContentStatus } from "../diagrams"

export const diagrams = sqliteTable("diagrams", {
  id: integer().primaryKey(),
  name: text().notNull(),
})

export const contents = sqliteTable("flow_contents", {
  diagramId: integer("diagram_id").notNull(),
  id: integer().notNull(),
  parentId: integer("parent_id"),
  kind: text().$type<ContentKind>().notNull(),
  name: text().notNull(),
  body: text(),
  status: text().$type<ContentStatus>(),
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
