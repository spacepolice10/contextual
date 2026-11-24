import { getPagePath, redirectPage } from "@nanostores/router"
import { atom, computed } from "nanostores"
import { define } from "nanotags"
import { renderList } from "nanotags/render"
import { mount } from "pele"
import { goalTextLimit, type Diagram } from "../diagrams"
import { libraryClient } from "../client/library"
import { $router } from "./router"
import { flowView, overlayRootButton, type FlowSelection } from "./flow"
import { inspectFlow, statusLabel } from "./inspector"
import "./styles.css"

declare global {
  interface HTMLElementTagNameMap {
    "contextual-library": InstanceType<typeof ContextualLibrary>
  }
}

type LibraryState =
  | { status: "loading" }
  | { status: "ready"; diagrams: Diagram[] }
  | { status: "error" }

interface GoalEditor {
  diagramId: number
  left: number
  top: number
  phase: "editing" | "saving" | "planning"
  goalId?: number
  error?: string
}

type Inspection = FlowSelection & { diagramId: number }

const ContextualLibrary = define("contextual-library")
  .withRefs((r) => ({
    list: r.one("ul"),
    item: r.one("template"),
    title: r.one("h1"),
    canvas: r.one("section"),
    message: r.one("p"),
    diagram: r.one("div"),
    goalForm: r.one("form"),
    goalText: r.one("textarea"),
    goalStatus: r.one("p"),
    goalCancel: r.one("button"),
    goalSubmit: r.one("button"),
    inspector: r.one("aside"),
    inspectorTitle: r.one("h2"),
    inspectorClose: r.one("button"),
    inspectorStatus: r.one("p"),
    inspectorBody: r.one("p"),
    inspectorList: r.one("ul"),
    inspectorItem: r.one("template"),
    inspectorAction: r.one("button"),
    inspectorAdd: r.one("form"),
    inspectorKind: r.one("select"),
    inspectorName: r.one("input"),
    inspectorAddStatus: r.one("p"),
  }))
  .setup((ctx) => {
    const {
      list, item, title, canvas, message, diagram, goalForm, goalText, goalStatus, goalCancel, goalSubmit,
      inspector, inspectorTitle, inspectorClose, inspectorStatus, inspectorBody, inspectorList, inspectorItem, inspectorAction, inspectorAdd, inspectorKind, inspectorName, inspectorAddStatus,
    } = ctx.refs
    const client = libraryClient()
    const $library = atom<LibraryState>({ status: "loading" })
    const $editor = atom<GoalEditor | null>(null)
    const $goalText = atom("")
    const $inspection = atom<Inspection | null>(null)
    const $actionState = atom<{ inspection: Inspection; pending: boolean; error?: string } | null>(null)
    const request = new AbortController()
    let libraryRequest = 0
    const $selected = computed([$library, $router], (state, page) => {
      if (state.status === "ready" && page?.route === "diagram") {
        return state.diagrams.find(({ id }) => String(id) === page.params.id)
      }
    })
    const $view = computed($selected, (selected) => selected?.flow ? flowView(selected.flow) : undefined)
    const $diagramId = computed($selected, (selected) => selected?.id)
    const $diagramName = computed($selected, (selected) => selected?.name)
    const $diagramSource = computed([$selected, $view], (selected, view) => view?.source ?? selected?.source)
    const $inspector = computed([$selected, $inspection], (selected, inspection) => {
      if (selected?.flow && selected.id === inspection?.diagramId) return inspectFlow(selected.flow, inspection)
    })

    async function loadLibrary(signal: AbortSignal) {
      const request = ++libraryRequest
      try {
        const diagrams = await client.list()
        if (signal.aborted || request !== libraryRequest) return

        const first = diagrams[0]
        if (first && $router.get()?.route === "library") {
          redirectPage($router, "diagram", { id: String(first.id) })
        }
        const state = $library.get()
        const previous = state.status === "ready" ? state.diagrams : []
        const next = diagrams.map((entry, index) => {
          const current = previous[index]
          return current?.id === entry.id && JSON.stringify(current) === JSON.stringify(entry) ? current : entry
        })
        if (state.status !== "ready" || previous.length !== next.length || next.some((entry, index) => entry !== previous[index])) {
          $library.set({ status: "ready", diagrams: next })
        }
      } catch (error) {
        console.error(error)
        if (!signal.aborted && request === libraryRequest && $library.get().status !== "ready") {
          $library.set({ status: "error" })
        }
      }
    }

    function openInspection(target: Element) {
      const selected = $selected.get()
      const id = target.closest("[data-flow-selection]")?.getAttribute("data-id")
      const selection = id ? $view.get()?.selections.get(id) : undefined
      if (selected && selection) $inspection.set({ diagramId: selected.id, ...selection })
    }

    function errorMessage(error: unknown, fallback: string) {
      return error instanceof Error ? error.message.replace(/^Error invoking remote method '[^']+': Error: /, "") : fallback
    }

    async function performAction() {
      const inspection = $inspection.get()
      const action = $inspector.get()?.action
      if (!inspection || !action || $actionState.get()?.pending) return
      $actionState.set({ inspection, pending: true })
      try {
        if (action.type === "plan_goal") {
          if (!client.planGoal) return
          await client.planGoal(inspection.diagramId, action.id)
        } else {
          await client.workflow(inspection.diagramId, action.type, action.id)
        }
        await loadLibrary(request.signal)
        $actionState.set(null)
      } catch (error) {
        $actionState.set({ inspection, pending: false, error: errorMessage(error, "Действие не выполнено.") })
      }
    }

    async function submitGoal() {
      const editor = $editor.get()
      const text = $goalText.get().trim()
      if (!editor || editor.phase !== "editing" || !text || text.length > goalTextLimit) return

      const current: GoalEditor = { ...editor, error: undefined, phase: "saving" }
      $editor.set(current)
      try {
        await client.createGoal(editor.diagramId, text)
        await loadLibrary(request.signal)
        if ($editor.get() === current) $editor.set(null)
      } catch (error) {
        if ($editor.get() !== current) return
        const message = errorMessage(error, "Не удалось сохранить цель.")
        $editor.set({ ...current, phase: "editing", error: message })
      }
    }

    ctx.bind($goalText, goalText)
    ctx.on(diagram, "click", (event) => {
      const target = event.target as Element
      const button = target.closest(".root-add-button")
      const selected = $selected.get()
      if (!button) {
        openInspection(target)
        return
      }
      if (!selected?.flow) return
      const editor = $editor.get()
      if (editor?.diagramId === selected.id) return

      const bounds = button.getBoundingClientRect()
      $goalText.set("")
      $editor.set({ diagramId: selected.id, left: bounds.right, top: bounds.bottom, phase: "editing" })
    })
    ctx.on(diagram, "keydown", (event) => {
      const target = event.target as Element
      if ((event.key === "Enter" || event.key === " ") && target.hasAttribute("data-flow-selection")) {
        event.preventDefault()
        openInspection(target)
      }
    })
    ctx.on(inspectorAdd, "submit", async (event) => {
      event.preventDefault()
      const selection = $inspection.get()
      const kind = inspectorKind.value
      const name = inspectorName.value.trim()
      if (!selection || selection.nodeId === undefined || !name) return
      if (kind !== "task" && kind !== "comment" && kind !== "solution" && kind !== "evidence") return
      inspectorAddStatus.textContent = ""
      try {
        await client.createNode(selection.diagramId, selection.nodeId, kind, name)
        inspectorName.value = ""
        await loadLibrary(request.signal)
      } catch (error) { inspectorAddStatus.textContent = errorMessage(error, "Не удалось добавить запись.") }
    })
    ctx.on(inspectorClose, "click", () => $inspection.set(null))
    ctx.on(inspectorAction, "click", () => void performAction())
    ctx.on(inspectorList, "click", (event) => {
      const selected = $selected.get()
      const id = Number((event.target as Element).closest("button[data-content-id]")?.getAttribute("data-content-id"))
      if (!selected?.flow || !Number.isSafeInteger(id) || id < 1) return
      const nodeId = selected.flow.nodes.find((node) => node.contentIds.length === 1 && node.contentIds[0] === id)?.id
      $inspection.set({ diagramId: selected.id, nodeId, contentIds: [id] })
    })
    ctx.on(goalForm, "submit", (event) => {
      event.preventDefault()
      void submitGoal()
    })
    ctx.on(goalCancel, "click", () => $editor.set(null))
    ctx.on(goalForm, "keydown", (event) => {
      if (event.key === "Escape" && $editor.get()?.phase === "editing") {
        event.preventDefault()
        $editor.set(null)
      }
    })
    ctx.effect([$editor, $selected, $goalText], (editor, selected, text) => {
      const visible = editor && editor.diagramId === selected?.id
      if (!visible) {
        if (goalForm.matches(":popover-open")) {
          goalForm.hidePopover()
          if (!editor) diagram.querySelector<HTMLButtonElement>(".root-add-button")?.focus()
        }
        return
      }

      const pending = editor.phase !== "editing"
      goalText.maxLength = goalTextLimit
      goalText.readOnly = pending || editor.goalId !== undefined
      goalCancel.disabled = pending
      goalSubmit.disabled = pending || !text.trim()
      goalSubmit.textContent = "Создать цель"
      goalStatus.textContent = pending
        ? (editor.phase === "saving" ? "Сохраняем цель…" : "Агент составляет список задач…")
        : editor.error ?? ""
      goalStatus.hidden = !goalStatus.textContent
      goalForm.style.setProperty("--goal-left", `${editor.left}px`)
      goalForm.style.setProperty("--goal-top", `${editor.top}px`)
      if (!goalForm.matches(":popover-open")) goalForm.showPopover()
    })

    ctx.effect([$inspector, $inspection, $actionState], (view, inspection, actionState) => {
      inspector.hidden = !view
      inspectorAdd.hidden = inspection?.nodeId === undefined
      if (!view) return
      const localAction = actionState?.inspection === inspection ? actionState : undefined
      inspectorTitle.textContent = view.title
      inspectorBody.textContent = view.body ?? ""
      inspectorBody.hidden = !view.body
      inspectorStatus.textContent = localAction?.error ?? (localAction?.pending ? "Выполняется…" : view.status ? statusLabel[view.status] : "")
      inspectorStatus.hidden = !inspectorStatus.textContent
      inspectorAction.hidden = !view.action || (["plan_goal", "queue_task", "retry_task"].includes(view.action.type) && !client.planGoal)
      inspectorAction.textContent = view.action?.label ?? ""
      inspectorAction.disabled = Boolean(actionState?.pending)
      inspectorList.hidden = !view.listed.length
      renderList(inspectorList, inspectorItem, {
        data: view.listed,
        key: ({ id }) => id,
        update: (row, content) => {
          ctx.getElement(row, "button").dataset.contentId = String(content.id)
          ctx.getElement(row, "span").textContent = content.name
          const status = ctx.getElement(row, "small")
          status.textContent = content.status ? statusLabel[content.status] : ""
          status.hidden = !status.textContent
        },
      })
    })

    ctx.effect([$library, $router, $selected], (state, page, selected) => {
      renderList(list, item, {
        data: state.status === "ready" ? state.diagrams.map(({ id, name }) => ({
          id,
          name,
          current: id === selected?.id,
        })) : [],
        key: ({ id }) => id,
        update: (row, { id, name, current }) => {
          const link = ctx.getElement(row, "a")
          link.textContent = name
          link.href = getPagePath($router, "diagram", { id: String(id) })
          if (current) link.setAttribute("aria-current", "page")
          else link.removeAttribute("aria-current")
        },
      })

      canvas.setAttribute("aria-busy", String(state.status === "loading"))
      if (state.status === "loading") message.textContent = "Loading diagrams…"
      else if (state.status === "error") message.textContent = "The diagram library could not be opened."
      else if (!page) message.textContent = "This page could not be found."
      else if (page.route === "diagram" && !selected) message.textContent = "This diagram could not be found."
      else if (state.diagrams.length === 0) message.textContent = "No diagrams in this library."
      else message.textContent = ""
      message.hidden = message.textContent === ""
    })

    ctx.effect([$diagramId, $diagramName, $diagramSource], (id, name, source) => {
      title.textContent = name ?? "Diagram library"
      diagram.hidden = id === undefined
      const flow = $selected.get()?.flow
      diagram.toggleAttribute("data-flow", Boolean(flow))
      if (source === undefined) return

      const root = flow?.nodes.find((node) => node.kind === "root")
      const preview = mount(diagram, source, {
        padding: 24,
        links: false,
        images: false,
        onRender: () => {
          const svg = diagram.querySelector("svg")
          if (!svg) return
          if (!root) svg.setAttribute("role", "img")
          svg.setAttribute("aria-label", name ?? "")
          if (root) overlayRootButton(svg, root)
          const view = $view.get()
          if (view) {
            for (const node of svg.querySelectorAll<SVGGElement>(".pele-node[data-id]")) {
              if (!view.selections.has(node.dataset.id ?? "")) continue
              node.setAttribute("data-flow-selection", "")
              node.setAttribute("tabindex", "0")
              node.setAttribute("role", "button")
              node.setAttribute("aria-label", node.textContent ?? "")
            }
          }
        },
      })

      return () => {
        preview.destroy()
        diagram.replaceChildren()
      }
    })

    ctx.onCleanup(() => request.abort())
    ctx.onCleanup(client.onChange(() => void loadLibrary(request.signal)))
    void loadLibrary(request.signal)
  })
