import type { DatabaseSync } from "node:sqlite"
import { contextualDevelopmentDiagram, contextualDevelopmentDiagramV2, starterDiagrams } from "./legacy-seed"
import { contextualDevelopmentFlow, starterFlows, wildberriesFlow, type SeedFlow } from "./seed"

function migrateToFlows(db: DatabaseSync, version: number) {
  const insertDiagram = db.prepare("INSERT INTO diagrams (name, source) VALUES (?, '')")
  const insertNode = db.prepare("INSERT INTO flow_nodes (diagram_id, id, parent_id, name) VALUES (?, ?, ?, ?)")
  const insertContent = db.prepare(`
    INSERT INTO flow_contents (diagram_id, id, parent_id, kind, name, body, status) VALUES (?, ?, ?, ?, ?, ?, ?)
  `)
  const linkContent = db.prepare(`
    INSERT INTO flow_node_contents (diagram_id, node_id, content_id, position) VALUES (?, ?, ?, 0)
  `)

  function writeFlow(flow: SeedFlow, diagramId = insertDiagram.run(flow.name).lastInsertRowid) {
    const rootId = 1
    insertNode.run(diagramId, rootId, null, flow.name)
    for (const { id, parentId, kind, name, body, status } of flow.contents) {
      insertContent.run(diagramId, id, parentId, kind, name, body ?? null, status ?? null)
      insertNode.run(diagramId, id, parentId ?? rootId, null)
      linkContent.run(diagramId, id, id)
    }
  }

  if (version === 0) {
    for (const flow of starterFlows) writeFlow(flow)
  } else {
    const legacyExamples = [...starterDiagrams, contextualDevelopmentDiagramV2, contextualDevelopmentDiagram]
    const flowsByName = new Map([...starterFlows, contextualDevelopmentFlow].map((flow) => [flow.name, flow]))
    const standalone = db.prepare(`
      SELECT id, name, source FROM diagrams
      WHERE NOT EXISTS (SELECT 1 FROM flow_nodes WHERE diagram_id = diagrams.id)
    `).all() as { id: number; name: string; source: string }[]

    for (const { id, name, source } of standalone) {
      const seed = flowsByName.get(name)
      const unchanged = legacyExamples.some((example) => example.name === name && example.source === source)
      if (seed && unchanged) {
        writeFlow(seed, id)
      } else {
        writeFlow({
          name,
          contents: source ? [
            { id: 2, parentId: null, kind: "goal", name: "Сохранить исходную схему", status: "approved" },
            { id: 3, parentId: 2, kind: "task", name: "Перенести исходный текст", status: "accepted" },
            { id: 4, parentId: 3, kind: "solution", name: "Исходник Mermaid", body: source, status: "accepted" },
          ] : [],
        }, id)
      }
    }
  }

  if (version < 2) writeFlow(contextualDevelopmentFlow)
  if (version < 4) writeFlow(wildberriesFlow)

  db.exec("ALTER TABLE diagrams DROP COLUMN source; PRAGMA user_version = 8;")
}

export function migrate(db: DatabaseSync) {
  const schemaVersion = db.prepare("PRAGMA user_version")
  if (Number(schemaVersion.get()?.user_version) < 8) {
    db.exec("BEGIN IMMEDIATE")
    // Another instance may have migrated the file while this one acquired the lock.
    const version = Number(schemaVersion.get()?.user_version)
    if (version === 0) {
      db.exec(`
        CREATE TABLE diagrams (
          id INTEGER PRIMARY KEY,
          title TEXT NOT NULL,
          source TEXT NOT NULL
        ) STRICT
      `)
    }
    if (version < 4) {
      db.exec(`
        CREATE TABLE diagram_nodes (
          diagram_id INTEGER NOT NULL REFERENCES diagrams(id) ON DELETE CASCADE,
          id INTEGER NOT NULL,
          parent_id INTEGER,
          kind TEXT NOT NULL CHECK (kind IN ('root', 'goal', 'task', 'comment', 'solution', 'evidence')),
          label TEXT NOT NULL,
          PRIMARY KEY (diagram_id, id),
          FOREIGN KEY (diagram_id, parent_id) REFERENCES diagram_nodes(diagram_id, id),
          CHECK ((kind = 'root') = (parent_id IS NULL)),
          CHECK (parent_id <> id)
        ) STRICT;

        CREATE UNIQUE INDEX diagram_nodes_root ON diagram_nodes(diagram_id) WHERE kind = 'root';
      `)
    } else if (version === 4) {
      const previousTitle = "Карта работы с агентом"
      const renamed = db.prepare(`
        UPDATE diagrams
        SET title = ?
        WHERE title = ? AND source = '' AND EXISTS (
          SELECT 1 FROM diagram_nodes
          WHERE diagram_id = diagrams.id AND kind = 'root' AND label = ?
        )
        RETURNING id
      `).all(wildberriesFlow.name, previousTitle, previousTitle) as { id: number }[]
      const renameRoot = db.prepare(`
        UPDATE diagram_nodes SET label = ? WHERE diagram_id = ? AND kind = 'root'
      `)
      for (const { id } of renamed) renameRoot.run(wildberriesFlow.name, id)
    }
    if (version < 6) {
      db.exec(`
        ALTER TABLE diagrams RENAME COLUMN title TO name;
        ALTER TABLE diagram_nodes RENAME COLUMN label TO name;
        PRAGMA user_version = 6;
      `)
    }
    if (version < 7) db.exec(`
      CREATE TABLE flow_contents (
        diagram_id INTEGER NOT NULL REFERENCES diagrams(id) ON DELETE CASCADE,
        id INTEGER NOT NULL,
        parent_id INTEGER,
        kind TEXT NOT NULL CHECK (kind IN ('goal','task','comment','solution','evidence')),
        name TEXT NOT NULL,
        body TEXT,
        status TEXT CHECK (status IN ('draft','approved','planned','queued','running','review','accepted','failed')),
        PRIMARY KEY (diagram_id,id),
        FOREIGN KEY (diagram_id,parent_id) REFERENCES flow_contents(diagram_id,id),
        CHECK (parent_id <> id)
      ) STRICT;

      CREATE TABLE flow_nodes (
        diagram_id INTEGER NOT NULL REFERENCES diagrams(id) ON DELETE CASCADE,
        id INTEGER NOT NULL,
        parent_id INTEGER,
        name TEXT,
        PRIMARY KEY (diagram_id,id),
        FOREIGN KEY (diagram_id,parent_id) REFERENCES flow_nodes(diagram_id,id),
        CHECK (parent_id <> id)
      ) STRICT;
      CREATE UNIQUE INDEX flow_nodes_root ON flow_nodes(diagram_id) WHERE parent_id IS NULL;

      CREATE TABLE flow_node_contents (
        diagram_id INTEGER NOT NULL,
        node_id INTEGER NOT NULL,
        content_id INTEGER NOT NULL,
        position INTEGER NOT NULL CHECK (position >= 0),
        PRIMARY KEY (diagram_id,node_id,content_id),
        UNIQUE (diagram_id,node_id,position),
        FOREIGN KEY (diagram_id,node_id) REFERENCES flow_nodes(diagram_id,id) ON DELETE CASCADE,
        FOREIGN KEY (diagram_id,content_id) REFERENCES flow_contents(diagram_id,id)
      ) STRICT;

      INSERT INTO flow_nodes(diagram_id,id,parent_id,name)
        SELECT diagram_id,id,parent_id,CASE WHEN kind='root' THEN name END FROM diagram_nodes;
      INSERT INTO flow_contents(diagram_id,id,parent_id,kind,name,status)
        SELECT n.diagram_id,n.id,CASE WHEN parent.kind <> 'root' THEN n.parent_id END,n.kind,n.name,
          CASE n.kind
            WHEN 'goal' THEN 'draft'
            WHEN 'task' THEN CASE WHEN EXISTS (
              SELECT 1 FROM diagram_nodes child WHERE child.diagram_id=n.diagram_id AND child.parent_id=n.id AND child.kind='solution'
            ) THEN 'review' ELSE 'planned' END
            WHEN 'solution' THEN 'review'
          END
        FROM diagram_nodes n LEFT JOIN diagram_nodes parent ON parent.diagram_id=n.diagram_id AND parent.id=n.parent_id
        WHERE n.kind <> 'root';
      INSERT INTO flow_node_contents(diagram_id,node_id,content_id,position)
        SELECT diagram_id,id,id,0 FROM diagram_nodes WHERE kind <> 'root';
      DROP TABLE diagram_nodes;
      CREATE INDEX flow_contents_parent ON flow_contents(diagram_id,parent_id);
      CREATE INDEX flow_contents_status ON flow_contents(kind,status);
      CREATE INDEX flow_node_contents_content ON flow_node_contents(diagram_id,content_id);
      PRAGMA user_version = 7;
    `)
    if (version < 8) migrateToFlows(db, version)
    db.exec("COMMIT")
  }
}
