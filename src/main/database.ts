import { mkdirSync } from "node:fs"
import { dirname } from "node:path"
import { DatabaseSync } from "node:sqlite"
import type { Diagram } from "../diagrams"
import { contextualDevelopmentDiagram, contextualDevelopmentDiagramV2, starterDiagrams } from "./seed"

export function listDiagrams(path: string): Diagram[] {
  mkdirSync(dirname(path), { recursive: true })
  const db = new DatabaseSync(path)

  try {
    const schemaVersion = db.prepare("PRAGMA user_version")
    if (Number(schemaVersion.get()?.user_version) < 3) {
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
        const insert = db.prepare("INSERT INTO diagrams (title, source) VALUES (?, ?)")
        for (const { title, source } of starterDiagrams) insert.run(title, source)
      }
      if (version < 2) {
        const { title, source } = contextualDevelopmentDiagram
        db.prepare("INSERT INTO diagrams (title, source) VALUES (?, ?)").run(title, source)
      } else if (version === 2) {
        const { title, source } = contextualDevelopmentDiagramV2
        db.prepare("UPDATE diagrams SET source = ? WHERE title = ? AND source = ?")
          .run(contextualDevelopmentDiagram.source, title, source)
      }
      if (version < 3) db.exec("PRAGMA user_version = 3")
      db.exec("COMMIT")
    }

    return db.prepare("SELECT id, title, source FROM diagrams ORDER BY id").all() as unknown as Diagram[]
  } finally {
    // Closing also rolls back a migration that threw before COMMIT.
    db.close()
  }
}
