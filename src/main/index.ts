import { app, BrowserWindow, ipcMain, Menu, type IpcMainInvokeEvent } from "electron"
import { watch, type FSWatcher } from "node:fs"
import { join } from "node:path"
import { createGoalChannel, diagramsChangedChannel, listDiagramsChannel, planGoalChannel, workflowActions, workflowChannel, type FlowNode, type WorkflowAction } from "../diagrams"
import { applyWorkflow, createGoal, initializeLibrary, readGoal, readLibrary, recoverInterruptedTasks, saveGoalTasks } from "./database"
import { generateGoalTasks } from "./agent-runner"
import { registerRendererScheme, rendererURL, serveRenderer } from "./renderer-protocol"
import { createTaskWorker } from "./task-worker"

declare const MAIN_WINDOW_VITE_DEV_SERVER_URL: string | undefined
declare const MAIN_WINDOW_VITE_NAME: string

const startURL = MAIN_WINDOW_VITE_DEV_SERVER_URL ?? rendererURL
const rendererLocation = new URL(startURL)

function assertRenderer(event: IpcMainInvokeEvent) {
  const frame = event.senderFrame
  if (!frame || frame !== event.sender.mainFrame || !BrowserWindow.fromWebContents(event.sender)) {
    throw new Error("Запрос доступен только из окна Contextual.")
  }
  const location = new URL(frame.url)
  if (location.protocol !== rendererLocation.protocol || location.host !== rendererLocation.host) {
    throw new Error("Запрос доступен только из окна Contextual.")
  }
}

function openWindow() {
  const window = new BrowserWindow({
    title: "Contextual",
    width: 1140,
    height: 780,
    minWidth: 420,
    minHeight: 480,
    backgroundColor: "#faf8f4",
    webPreferences: {
      preload: join(__dirname, "preload.cjs"),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
    },
  })

  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }))
  window.webContents.on("will-navigate", (event) => {
    const destination = new URL(event.url)
    if (destination.protocol !== rendererLocation.protocol || destination.host !== rendererLocation.host) {
      event.preventDefault()
    }
  })

  void window.loadURL(startURL).catch(console.error)
}

async function start() {
  await app.whenReady()
  if (!MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    serveRenderer(join(__dirname, "../renderer", MAIN_WINDOW_VITE_NAME))
  }

  const databasePath = join(app.getPath("userData"), "diagrams.sqlite")
  const activePlans = new Map<string, Promise<FlowNode[]>>()
  const agentRuns = new AbortController()
  const worker = createTaskWorker(databasePath, agentRuns.signal)
  app.on("before-quit", () => agentRuns.abort())
  let ready: Promise<void> | undefined
  let watcher: FSWatcher | undefined

  function openLibrary() {
    if (!ready) {
      initializeLibrary(databasePath)
      ready = recoverInterruptedTasks(databasePath).then(() => worker.wake()).catch((error) => {
        ready = undefined
        throw error
      })
    }
    return ready
  }

  ipcMain.handle(listDiagramsChannel, async (event) => {
    assertRenderer(event)
    await openLibrary()
    if (!watcher) {
      watcher = watch(databasePath, { persistent: false }, () => {
        worker.wake()
        for (const window of BrowserWindow.getAllWindows()) {
          if (!window.webContents.isDestroyed()) window.webContents.send(diagramsChangedChannel)
        }
      })
      watcher.on("error", () => {
        watcher?.close()
        watcher = undefined
      })
    }
    return readLibrary(databasePath)
  })
  app.on("will-quit", () => watcher?.close())
  ipcMain.handle(createGoalChannel, async (event, diagramId: number, text: string) => {
    assertRenderer(event)
    await openLibrary()
    return createGoal(databasePath, diagramId, text)
  })
  ipcMain.handle(planGoalChannel, async (event, diagramId: number, goalId: number) => {
    assertRenderer(event)
    await openLibrary()
    const { context, hasTasks } = await readGoal(databasePath, diagramId, goalId)
    if (hasTasks) return []

    const key = `${diagramId}:${goalId}`
    const active = activePlans.get(key)
    if (active) return active

    const plan = generateGoalTasks(context, agentRuns.signal)
      .then((names) => saveGoalTasks(databasePath, diagramId, goalId, names))
      .finally(() => activePlans.delete(key))
    activePlans.set(key, plan)
    return plan
  })
  ipcMain.handle(workflowChannel, async (event, diagramId: number, action: WorkflowAction, contentId: number) => {
    assertRenderer(event)
    if (!Number.isSafeInteger(diagramId) || !Number.isSafeInteger(contentId) || !workflowActions.includes(action)) {
      throw new Error("Действие недоступно.")
    }
    await openLibrary()
    await applyWorkflow(databasePath, diagramId, action, contentId)
    worker.wake()
  })

  if (process.platform !== "darwin") Menu.setApplicationMenu(null)

  app.on("second-instance", () => {
    const [window] = BrowserWindow.getAllWindows()
    if (!window) {
      openWindow()
      return
    }
    if (window.isMinimized()) window.restore()
    window.focus()
  })

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) openWindow()
  })

  app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit()
  })

  openWindow()
}

if (app.requestSingleInstanceLock()) {
  registerRendererScheme()
  void start()
} else {
  app.quit()
}
