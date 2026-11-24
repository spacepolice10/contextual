import { contextBridge, ipcRenderer } from "electron"
import { createGoalChannel, createNodeChannel, diagramsChangedChannel, listDiagramsChannel, planGoalChannel, workflowChannel, type DiagramLibrary } from "./diagrams"

const diagrams: DiagramLibrary = {
  list: () => ipcRenderer.invoke(listDiagramsChannel),
  createGoal: (diagramId, text) => ipcRenderer.invoke(createGoalChannel, diagramId, text),
  createNode: (diagramId, parentId, kind, name) => ipcRenderer.invoke(createNodeChannel, diagramId, parentId, kind, name),
  planGoal: (diagramId, goalId) => ipcRenderer.invoke(planGoalChannel, diagramId, goalId),
  workflow: (diagramId, action, contentId) => ipcRenderer.invoke(workflowChannel, diagramId, action, contentId),
  onChange: (listener) => {
    const forward = () => listener()
    ipcRenderer.on(diagramsChangedChannel, forward)
    return () => ipcRenderer.removeListener(diagramsChangedChannel, forward)
  },
}

contextBridge.exposeInMainWorld("diagrams", diagrams)
