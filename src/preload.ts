import { contextBridge, ipcRenderer } from "electron"
import { listDiagramsChannel, type DiagramLibrary } from "./diagrams"

const diagrams: DiagramLibrary = {
  list: () => ipcRenderer.invoke(listDiagramsChannel),
}

contextBridge.exposeInMainWorld("diagrams", diagrams)
