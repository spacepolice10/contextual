import { app, BrowserWindow, ipcMain, Menu, type IpcMainInvokeEvent } from "electron"
import { watch, type FSWatcher } from "node:fs"
import { join } from "node:path"
import { createGoalChannel, createNodeChannel, diagramsChangedChannel, listDiagramsChannel, planGoalChannel, workflowActions, workflowChannel, type ContentKind, type WorkflowAction } from "../diagrams"
import { recoverInterruptedTasks } from "./database"
import { createLibrary, createPlanningService } from "../application/library"
import { generateGoalTasks, completeTask } from "./agent-runner"
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
  const library = createLibrary(databasePath)
  const agentRuns = new AbortController()
  const worker = createTaskWorker(databasePath, agentRuns.signal, completeTask)
  const planGoal = createPlanningService(databasePath, generateGoalTasks, agentRuns.signal)
  app.on("before-quit", () => agentRuns.abort())
  let ready: Promise<void> | undefined
  let watcher: FSWatcher | undefined

  function openLibrary() {
    if (!ready) {
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
    return library.list()
  })
  app.on("will-quit", () => watcher?.close())
  ipcMain.handle(createGoalChannel, async (event, diagramId: number, text: string) => {
    assertRenderer(event)
    await openLibrary()
    return library.createGoal(diagramId, text)
  })
  ipcMain.handle(planGoalChannel, async (event, diagramId: number, goalId: number) => {
    assertRenderer(event)
    await openLibrary()
    return planGoal(diagramId, goalId)
  })
  ipcMain.handle(createNodeChannel, async (event, diagramId: number, parentId: number, kind: ContentKind, name: string) => {
    assertRenderer(event)
    await openLibrary()
    return library.createNode(diagramId, parentId, kind, name)
  })
  ipcMain.handle(workflowChannel, async (event, diagramId: number, action: WorkflowAction, contentId: number) => {
    assertRenderer(event)
    if (!Number.isSafeInteger(diagramId) || !Number.isSafeInteger(contentId) || !workflowActions.includes(action)) {
      throw new Error("Действие недоступно.")
    }
    await openLibrary()
    await library.workflow(diagramId, action, contentId)
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
