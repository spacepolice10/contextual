export interface Diagram {
  id: number
  title: string
  source: string
}

export interface DiagramLibrary {
  list: () => Promise<Diagram[]>
}

export const listDiagramsChannel = "diagrams:list"
