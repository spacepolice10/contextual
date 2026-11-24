import type { DatabaseSync } from "node:sqlite"

// Version 8: keep the existing content and placement IDs, and backfill only
// information that can actually be recovered from the old database.
export function migrateWorkflow(db: DatabaseSync) {
  db.exec(`
    ALTER TABLE flow_contents ADD COLUMN position INTEGER CHECK (position >= 0);
    ALTER TABLE flow_contents ADD COLUMN execution_stage INTEGER CHECK (execution_stage >= 0);

    CREATE TABLE agents_work (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      diagram_id INTEGER NOT NULL,
      content_id INTEGER NOT NULL,
      kind TEXT NOT NULL CHECK (kind IN ('planning','execution','revision')),
      status TEXT NOT NULL CHECK (status IN ('queued','running','completed','failed','cancelled','interrupted')),
      agent TEXT NOT NULL,
      input TEXT,
      instruction TEXT,
      error TEXT,
      retry_of_id INTEGER REFERENCES agents_work(id),
      base_version_id INTEGER REFERENCES content_versions(id),
      result_content_id INTEGER,
      created_at TEXT NOT NULL,
      started_at TEXT,
      finished_at TEXT,
      FOREIGN KEY (diagram_id,content_id) REFERENCES flow_contents(diagram_id,id) ON DELETE CASCADE,
      FOREIGN KEY (diagram_id,result_content_id) REFERENCES flow_contents(diagram_id,id)
    ) STRICT;

    CREATE TABLE content_versions (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      diagram_id INTEGER NOT NULL,
      content_id INTEGER NOT NULL,
      parent_version_id INTEGER REFERENCES content_versions(id),
      agents_work_id INTEGER REFERENCES agents_work(id),
      name TEXT NOT NULL,
      body TEXT,
      created_at TEXT NOT NULL,
      FOREIGN KEY (diagram_id,content_id) REFERENCES flow_contents(diagram_id,id) ON DELETE CASCADE
    ) STRICT;

    ALTER TABLE flow_contents ADD COLUMN latest_version_id INTEGER REFERENCES content_versions(id);
    ALTER TABLE flow_contents ADD COLUMN selected_version_id INTEGER REFERENCES content_versions(id);

    WITH ordered AS (
      SELECT diagram_id,id,row_number() OVER (PARTITION BY diagram_id,parent_id ORDER BY id)-1 AS pos
      FROM flow_contents WHERE kind='task'
    )
    UPDATE flow_contents SET position=ordered.pos,execution_stage=ordered.pos
    FROM ordered WHERE flow_contents.diagram_id=ordered.diagram_id AND flow_contents.id=ordered.id;

    INSERT INTO content_versions(diagram_id,content_id,name,body,created_at)
      SELECT diagram_id,id,name,body,strftime('%Y-%m-%dT%H:%M:%fZ','now')
      FROM flow_contents WHERE kind='solution';

    UPDATE flow_contents SET latest_version_id=(
      SELECT v.id FROM content_versions v WHERE v.diagram_id=flow_contents.diagram_id AND v.content_id=flow_contents.id
    ) WHERE kind='solution';
    UPDATE flow_contents SET selected_version_id=latest_version_id WHERE kind='solution' AND status='accepted';

    INSERT INTO agents_work(diagram_id,content_id,kind,status,agent,error,created_at)
      SELECT diagram_id,id,'execution',status,'codex',
        CASE WHEN status='failed' THEN 'Failure recorded before workflow migration.' END,
        strftime('%Y-%m-%dT%H:%M:%fZ','now')
      FROM flow_contents WHERE kind='task' AND status IN ('queued','running','failed');

    CREATE INDEX task_stage_order ON flow_contents(diagram_id,parent_id,execution_stage,position);
    CREATE INDEX content_versions_content ON content_versions(diagram_id,content_id,id);
    CREATE INDEX agents_work_queue ON agents_work(status,kind,id);
    CREATE INDEX agents_work_content ON agents_work(diagram_id,content_id,id);
    CREATE UNIQUE INDEX agents_work_active_execution ON agents_work(diagram_id,content_id)
      WHERE kind='execution' AND status IN ('queued','running');
    CREATE UNIQUE INDEX agents_work_active_planning ON agents_work(diagram_id,content_id)
      WHERE kind='planning' AND status IN ('queued','running');

    CREATE TRIGGER content_versions_immutable BEFORE UPDATE ON content_versions
      BEGIN SELECT RAISE(ABORT,'Content versions are immutable'); END;

    CREATE TABLE change_events (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      diagram_id INTEGER NOT NULL,
      entity TEXT NOT NULL CHECK (entity IN ('flow','content','node','version','agents_work')),
      entity_id INTEGER NOT NULL,
      operation TEXT NOT NULL CHECK (operation IN ('created','updated','deleted')),
      created_at TEXT NOT NULL
    ) STRICT;
  `)

  // Triggers cover all writers, including a separate stdio MCP process. Their
  // journal entries are committed or rolled back together with the change.
  const tables = [
    ["diagrams", "flow", "id", "id"],
    ["flow_contents", "content", "diagram_id", "id"],
    ["flow_nodes", "node", "diagram_id", "id"],
    ["flow_node_contents", "node", "diagram_id", "node_id"],
    ["content_versions", "version", "diagram_id", "id"],
    ["agents_work", "agents_work", "diagram_id", "id"],
  ] as const
  for (const [table, entity, flow, id] of tables) {
    for (const [operation, event, row] of [
      ["INSERT", "created", "NEW"], ["UPDATE", "updated", "NEW"], ["DELETE", "deleted", "OLD"],
    ] as const) {
      db.exec(`CREATE TRIGGER changes_${table}_${operation.toLowerCase()} AFTER ${operation} ON ${table}
        BEGIN
          INSERT INTO change_events(diagram_id,entity,entity_id,operation,created_at)
          VALUES(${row}.${flow},'${entity}',${row}.${id},'${event}',strftime('%Y-%m-%dT%H:%M:%fZ','now'));
        END;`)
    }
  }
  db.exec("PRAGMA user_version = 8")
}
