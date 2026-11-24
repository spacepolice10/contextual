import { DatabaseSync, type SQLInputValue } from "node:sqlite"
import { drizzle, type SqliteRemoteDatabase } from "drizzle-orm/sqlite-proxy"

export type LibraryDatabase = SqliteRemoteDatabase

function connect(client: DatabaseSync) {
  // Drizzle's stable SQLite proxy works with Electron's bundled node:sqlite.
  return drizzle(async (query, params: SQLInputValue[], method) => {
    const statement = client.prepare(query)
    if (method === "run") {
      statement.run(...params)
      return { rows: [] }
    }
    statement.setReturnArrays(true)
    if (method === "get") return { rows: statement.get(...params) as unknown as [] }
    return { rows: statement.all(...params) }
  })
}

export async function read<T>(path: string, query: (db: LibraryDatabase) => Promise<T>) {
  const client = new DatabaseSync(path, { readOnly: true, timeout: 5000 })
  try {
    client.exec("BEGIN")
    return await query(connect(client))
  } finally {
    client.close()
  }
}

const writes = new Map<string, Promise<unknown>>()

export async function saveFlow<T>(path: string, write: (db: LibraryDatabase) => Promise<T>) {
  const pending = (writes.get(path) ?? Promise.resolve()).catch(() => {}).then(async () => {
    const client = new DatabaseSync(path, { timeout: 5000 })
    try {
      client.exec("BEGIN IMMEDIATE")
      const result = await write(connect(client))
      client.exec("COMMIT")
      return result
    } finally {
      client.close()
    }
  })
  writes.set(path, pending)
  try {
    return await pending
  } finally {
    if (writes.get(path) === pending) writes.delete(path)
  }
}
