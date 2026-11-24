import { completeTask } from "./agent-runner"
import { claimTask, failTask, saveTaskResult } from "./database"

export function createTaskWorker(databasePath: string, signal: AbortSignal) {
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
          void completeTask(task.context, signal)
            .then((result) => saveTaskResult(databasePath, task.diagramId, task.id, result))
            .catch(async () => {
              if (!signal.aborted) await failTask(databasePath, task.diagramId, task.id)
            })
            .catch(console.error)
            .finally(() => {
              active--
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
