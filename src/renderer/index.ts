import { getPagePath, redirectPage } from "@nanostores/router"
import { atom, computed } from "nanostores"
import { define } from "nanotags"
import { renderList } from "nanotags/render"
import { mount } from "pele"
import { goalTextLimit, type Diagram, type DiagramLibrary } from "../diagrams"
import { $router } from "./router"
import { flowView, overlayRootButton, type FlowSelection } from "./flow"
import { focusedFlow } from "./focused-view"
import { inspectFlow, statusLabel } from "./inspector"
import "./styles.css"

declare global {
  interface Window {
    diagrams: DiagramLibrary
  }

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
    focusExit: r.one("a"),
    focusCopy: r.one("button"),
    inspectorFocus: r.one("a"),
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
  }))
  .setup((ctx) => {
    const {
      list, item, title, focusExit, focusCopy, inspectorFocus, canvas, message, diagram, goalForm, goalText, goalStatus, goalCancel, goalSubmit,
      inspector, inspectorTitle, inspectorClose, inspectorStatus, inspectorBody, inspectorList, inspectorItem, inspectorAction,
    } = ctx.refs
    const $library = atom<LibraryState>({ status: "loading" })
    const $editor = atom<GoalEditor | null>(null)
    const $goalText = atom("")
    const $inspection = atom<Inspection | null>(null)
    const $actionState = atom<{ inspection: Inspection; pending: boolean; error?: string } | null>(null)
    const request = new AbortController()
    let libraryRequest = 0
    const $selected = computed([$library, $router], (state, page) => {
      if (state.status === "ready" && (page?.route === "diagram" || page?.route === "focused")) {
        return state.diagrams.find(({ id }) => String(id) === page.params.id)
      }
    })
    const $focusedId = computed($router, (page) => page?.route === "focused" ? Number(page.params.nodeId) : undefined)
    const $visibleFlow = computed([$selected, $focusedId], (selected, nodeId) => {
      if (!selected?.flow) return
      return nodeId === undefined ? selected.flow : focusedFlow(selected.flow, nodeId)
    })
    const $view = computed($visibleFlow, (flow) => flow ? flowView(flow) : undefined)
    const $diagramId = computed($selected, (selected) => selected?.id)
    const $diagramName = computed([$selected, $focusedId, $visibleFlow], (selected, nodeId, flow) => nodeId === undefined ? selected?.name : flow?.nodes.find((node) => node.id === nodeId)?.name ?? selected?.name)
    const $diagramSource = computed([$selected, $view, $focusedId], (selected, view, nodeId) => nodeId === undefined ? view?.source ?? selected?.source : view?.source)
    const $inspector = computed([$selected, $inspection], (selected, inspection) => {
      if (selected?.flow && selected.id === inspection?.diagramId) return inspectFlow(selected.flow, inspection)
    })

    async function loadLibrary(signal: AbortSignal) {
      const request = ++libraryRequest
      try {
        const diagrams = await window.diagrams.list()
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
          await window.diagrams.planGoal(inspection.diagramId, action.id)
        } else {
          await window.diagrams.workflow(inspection.diagramId, action.type, action.id)
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

      let current: GoalEditor = { ...editor, error: undefined, phase: editor.goalId ? "planning" : "saving" }
      $editor.set(current)
      try {
        let goalId = editor.goalId
        if (goalId === undefined) {
          const goal = await window.diagrams.createGoal(editor.diagramId, text)
          goalId = goal.id
          await loadLibrary(request.signal)
          if ($editor.get() === current) {
            current = { ...current, goalId, phase: "planning" }
            $editor.set(current)
          }
        }
        await window.diagrams.planGoal(editor.diagramId, goalId)
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
    ctx.effect([$router], () => {
      $inspection.set(null)
      $editor.set(null)
      focusCopy.textContent = "Copy link"
    })
    ctx.on(focusCopy, "click", async () => {
      const url = window.location.href
      try {
        await navigator.clipboard.writeText(url)
        if (window.location.href === url) focusCopy.textContent = "Copied"
      } catch {
        if (window.location.href === url) focusCopy.textContent = "Copy failed"
      }
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
      goalSubmit.textContent = editor.goalId && !pending ? "Повторить" : "Отправить в Codex"
      goalStatus.textContent = pending
        ? (editor.phase === "saving" ? "Сохраняем цель…" : "Codex составляет список задач…")
        : editor.error ?? ""
      goalStatus.hidden = !goalStatus.textContent
      goalForm.style.setProperty("--goal-left", `${editor.left}px`)
      goalForm.style.setProperty("--goal-top", `${editor.top}px`)
      if (!goalForm.matches(":popover-open")) goalForm.showPopover()
    })

    ctx.effect([$inspector, $inspection, $actionState, $focusedId, $visibleFlow], (view, inspection, actionState, focusedId, visibleFlow) => {
      inspector.hidden = !view
      if (!view) return
      const nodeId = inspection?.nodeId
      inspectorFocus.hidden = !inspection || nodeId === undefined || nodeId === focusedId || !visibleFlow?.nodes.some((node) => node.id === nodeId)
      if (inspection && nodeId !== undefined && !inspectorFocus.hidden) inspectorFocus.href = getPagePath($router, "focused", { id: String(inspection.diagramId), nodeId: String(nodeId) })
      else inspectorFocus.removeAttribute("href")
      const localAction = actionState?.inspection === inspection ? actionState : undefined
      inspectorTitle.textContent = view.title
      inspectorBody.textContent = view.body ?? ""
      inspectorBody.hidden = !view.body
      inspectorStatus.textContent = localAction?.error ?? (localAction?.pending ? "Выполняется…" : view.status ? statusLabel[view.status] : "")
      inspectorStatus.hidden = !inspectorStatus.textContent
      inspectorAction.hidden = !view.action
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

    ctx.effect([$library, $router, $selected, $visibleFlow], (state, page, selected, visibleFlow) => {
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
      else if ((page.route === "diagram" || page.route === "focused") && !selected) message.textContent = "This diagram could not be found."
      else if (page.route === "focused" && !visibleFlow) message.textContent = "This focused view could not be found."
      else if (state.diagrams.length === 0) message.textContent = "No diagrams in this library."
      else message.textContent = ""
      message.hidden = message.textContent === ""
    })

    ctx.effect([$selected, $focusedId], (selected, nodeId) => {
      const focused = selected !== undefined && nodeId !== undefined
      focusExit.hidden = !focused
      focusCopy.hidden = !focused
      if (focused) focusExit.href = getPagePath($router, "diagram", { id: String(selected.id) })
      else focusExit.removeAttribute("href")
    })

    ctx.effect([$diagramId, $diagramName, $diagramSource], (id, name, source) => {
      title.textContent = name ?? "Diagram library"
      diagram.hidden = id === undefined || source === undefined
      const flow = $visibleFlow.get()
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
          if (!flow) svg.setAttribute("role", "img")
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
    ctx.onCleanup(window.diagrams.onChange(() => void loadLibrary(request.signal)))
    void loadLibrary(request.signal)
  })
