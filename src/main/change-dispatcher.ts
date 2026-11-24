import { DatabaseSync } from "node:sqlite"
import type { ChangeEvent } from "../diagrams"

// A durable cursor, not a filesystem notification. External MCP writes are
// discovered even when SQLite uses WAL. No new events are written by polling.
export function createChangeDispatcher(path: string, publish: (events: ChangeEvent[]) => void) {
  let client: DatabaseSync | undefined
  let timer: ReturnType<typeof setInterval> | undefined
  let cursor = 0

  function wake() {
    if (!client) return
    try {
      const events = client.prepare(`SELECT id,diagram_id AS flowId,entity,
        entity_id AS entityId,operation,created_at AS createdAt
        FROM change_events WHERE id > ? ORDER BY id LIMIT 500`).all(cursor) as unknown as ChangeEvent[]
      if (events.length) {
        publish(events)
        cursor = events[events.length - 1].id
      }
    } catch (error) {
      console.error("Contextual change dispatcher:", error)
    }
  }

  async function start() {
    if (client) return
    client = new DatabaseSync(path, { readOnly: true, timeout: 5000 })
    cursor = Number(client.prepare("SELECT coalesce(max(id),0) AS id FROM change_events").get()?.id ?? 0)
    timer = setInterval(wake, 500)
    timer.unref()
  }

  function stop() {
    if (timer) clearInterval(timer)
    timer = undefined
    client?.close()
    client = undefined
  }

  return { start, stop, wake }
}
