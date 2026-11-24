import { completeTask } from "./agent-runner"
import { claimTask, failWork, saveTaskResult } from "./database"

export function createTaskWorker(databasePath: string, signal: AbortSignal, onChange: () => void = () => {}) {
  let active = 0
  let draining = false
  let requested = false
  const concurrency = 2

  async function drain() {
    try {
      while (requested && !signal.aborted) {
        requested = false
        while (active < concurrency && !signal.aborted) {
          const task = await claimTask(databasePath)
          if (!task) break
          active++
          onChange()
          void completeTask(task.context, signal)
            .then((result) => saveTaskResult(databasePath, task.id, result))
            .catch(async (error: unknown) => {
              if (!signal.aborted) await failWork(databasePath, task.id, error instanceof Error ? error.message : String(error))
            })
            .catch(console.error)
            .finally(() => {
              active--
              onChange()
              wake()
            })
        }
      }
    } finally {
      draining = false
    }
  }

  function wake() {
    if (signal.aborted) return
    requested = true
    if (!draining) {
      draining = true
      void drain().catch(console.error)
    }
  }

  return { wake }
}
