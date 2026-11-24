import { contextBridge, ipcRenderer } from "electron"
import { createGoalChannel, getDiagramChannel, diagramsChangedChannel, listDiagramsChannel, planGoalChannel, workflowChannel, type ChangeEvent, type DiagramLibrary } from "./diagrams"

const diagrams: DiagramLibrary = {
  list: () => ipcRenderer.invoke(listDiagramsChannel),
  get: (id) => ipcRenderer.invoke(getDiagramChannel, id),
  createGoal: (diagramId, text) => ipcRenderer.invoke(createGoalChannel, diagramId, text),
  planGoal: (diagramId, goalId) => ipcRenderer.invoke(planGoalChannel, diagramId, goalId),
  workflow: (diagramId, action, contentId) => ipcRenderer.invoke(workflowChannel, diagramId, action, contentId),
  onChange: (listener) => {
    const forward = (_event: unknown, events: ChangeEvent[]) => listener(events)
    ipcRenderer.on(diagramsChangedChannel, forward)
    return () => ipcRenderer.removeListener(diagramsChangedChannel, forward)
  },
}

contextBridge.exposeInMainWorld("diagrams", diagrams)
