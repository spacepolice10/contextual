import type { DiagramLibrary } from "../diagrams"
let client: DiagramLibrary | undefined
export function configureLibrary(value: DiagramLibrary) { client = value }
export function libraryClient(): DiagramLibrary {
  if (!client) throw new Error("Library client not configured")
  return client
}
