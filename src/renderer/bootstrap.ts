import type { DiagramLibrary } from "../diagrams"
import { configureLibrary } from "../client/library"
import { createHttpClient } from "../client/http"
configureLibrary((window as Window & { diagrams?: DiagramLibrary }).diagrams ?? createHttpClient())
void import("./index")
