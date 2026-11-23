import { app, BrowserWindow, ipcMain, Menu } from "electron"
import { join } from "node:path"
import { listDiagramsChannel } from "../diagrams"
import { listDiagrams } from "./database"

declare const MAIN_WINDOW_VITE_DEV_SERVER_URL: string | undefined
declare const MAIN_WINDOW_VITE_NAME: string

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
  window.webContents.on("will-navigate", (event) => event.preventDefault())

  const page = MAIN_WINDOW_VITE_DEV_SERVER_URL
    ? window.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL)
    : window.loadFile(join(__dirname, "../renderer", MAIN_WINDOW_VITE_NAME, "index.html"))
  void page.catch(console.error)
}

void app.whenReady().then(() => {
  ipcMain.handle(listDiagramsChannel, (event) => {
    if (!event.senderFrame || event.senderFrame !== event.sender.mainFrame) {
      throw new Error("Only the main window can read the diagram library.")
    }
    return listDiagrams(join(app.getPath("userData"), "diagrams.sqlite"))
  })

  if (process.platform !== "darwin") Menu.setApplicationMenu(null)
  openWindow()

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) openWindow()
  })
})

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit()
})
