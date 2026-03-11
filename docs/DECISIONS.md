# Architecture Decision Log

This document records the key design decisions made during rataframe's planning phase, along with the rationale and alternatives considered.

---

## ADR-001: Next.js Philosophy, Not Rails or Express

**Decision:** Follow Next.js-style "strong conventions as the happy path, clean escape hatches when they don't fit."

**Context:** We considered three philosophies:

| Approach | Example | Pros | Cons |
|----------|---------|------|------|
| Express (minimal) | Primitives, compose yourself | Maximum flexibility | Solves nothing — every app still writes 500 lines of boilerplate |
| Rails (opinionated) | Mandatory conventions | Maximum productivity for apps that fit | TUI apps are too diverse — editors, dashboards, file managers all differ fundamentally |
| Next.js (progressive) | Conventions as defaults, escape hatches | Best of both worlds | Slightly more complex framework internals |

**Why not Rails?** TUI apps are not as homogeneous as web apps. A system monitor, a text editor, and a file manager have fundamentally different architectures. Mandatory Panel trait would fight editor-style apps that use modes and buffers instead of panels.

**Why not Express?** Giving developers primitives without conventions means they still write their own focus manager, footer, help screen, event loop. We solve nothing meaningful.

**Result:** Convention path (Panel trait + auto-footer + auto-help + command palette) gives Rails-level productivity for the 80% of apps that fit. Custom path (override `view()`) gives full control for the 20% that don't.

---

## ADR-002: Panel Trait as Convention, Not Requirement

**Decision:** The `Panel` trait is the recommended way to build apps, but `Application::view()` remains as an escape hatch for apps that don't use panels.

**Context:** We debated whether panels should be mandatory (like Rails' MVC) or optional (like Next.js' file routing).

**Rationale:**
- Dashboard apps, list/detail apps, and most TUI tools fit the panel model perfectly.
- Editor-style apps (helix, vim) use buffers/splits/modes — forcing them into panels would be hostile.
- A 10-line hello-world app should not need to define a Panel.

**Result:** If you implement `panels()`, you get 10 free behaviors (focus cycling, auto-footer, auto-help, command palette, themed borders, mouse support, zoom, lifecycle hooks, Esc chain, convention keys). If you don't, you implement `view()` directly and still get the runtime, commands, overlays, and themes.

---

## ADR-003: Event Flow — App First, Panel Second, Convention Last

**Decision:** Events flow: overlay → app.handle_event → focused panel.handle_key → convention keys.

**Alternatives considered:**
1. Panel first, then app — good for component isolation but prevents global shortcuts
2. Convention first, then app — prevents overriding convention keys
3. App first, then panel, then convention — app has full control, panel handles its domain, convention is the fallback

**Rationale:** The app needs to intercept events before panels for global shortcuts (e.g., Ctrl+S to save regardless of focused panel). Panels handle their domain-specific keys. Convention keys are the safety net. Apps can suppress any level by returning `EventResult::Consumed`.

---

## ADR-004: Commands as Values (Iced-Inspired)

**Decision:** Side-effects are described as `Command` return values from `update()`, not executed inline.

**Context:** In Vortix, some side-effects are executed directly in `update()` (spawning threads, sending to channels). Iced and Elm prove that returning side-effects as values is more testable and composable.

**Rationale:**
- `Command::perform(future, mapper)` wraps any async work without the app needing to manage threads or channels
- `Command::batch([...])` composes multiple effects from a single state transition
- The runtime handles execution, threading, and message routing
- Tests can inspect returned Commands without executing them

---

## ADR-005: Tick and Terminal Events Are Built-In

**Decision:** The tick timer and crossterm event polling are handled by the runtime automatically. They are NOT subscriptions the app must set up.

**Alternative:** Iced treats keyboard events as subscriptions. We could do the same.

**Rationale:** Every TUI app needs terminal events and most need a tick. Making them subscriptions adds boilerplate for zero benefit. The `tick_rate()` method controls the interval. App-specific streams (file watchers, websockets) use `subscription()`.

---

## ADR-006: Screen Stack Instead of URL Routing

**Decision:** Navigation is a push/pop screen stack (like iOS UINavigationController), not URL-based routing (like React Router).

**Context:** Web apps need URL routing for deep linking, SEO, and browser history. TUI apps have none of these.

**Rationale:**
- Most TUI apps are single-screen with panels — no navigation needed at all
- The few that need screens (settings page, detail views) fit a push/pop model
- Panels + overlays already cover what routing covers in web apps (modals, detail views)
- The Esc convention chain integrates naturally: pop overlay → pop screen → quit

**Result:** `Command::push_screen(screen)` and `Command::pop_screen()` with lifecycle hooks (`on_enter`, `on_leave`). Simple, covers all TUI navigation needs.

---

## ADR-007: Auto-Generated Help and Command Palette

**Decision:** The Help overlay and Command Palette are auto-generated from all panels' `key_hints()` at runtime.

**Rationale:** This is rataframe's killer differentiator. No other TUI framework does this.

- Developers define `key_hints()` once per panel
- The footer shows hints for the focused panel (auto-generated)
- `?` opens a Help overlay grouping all hints by panel title (auto-generated)
- `:` opens a Command Palette with fuzzy search across all actions (auto-generated)
- Zero extra code from the developer for all three features

---

## ADR-008: Error Boundaries Per Panel

**Decision:** Each panel's `view()` call is wrapped in `std::panic::catch_unwind`. If a panel panics during rendering, the framework shows "Panel Error" in that panel's area and continues running.

**Rationale:** A framework should be more resilient than raw code. One buggy panel should not crash the entire application. This mirrors React's ErrorBoundary concept.

---

## ADR-009: One Crate, All Features

**Decision:** Ship everything in a single `rataframe` crate. No ecosystem of 10+ sub-crates.

**Alternative:** rat-salsa uses `rat-widget`, `rat-focus`, `rat-event`, `rat-ftable`, `rat-scrolled`, `rat-menu`, `rat-text`, `rat-dialog`, `rat-popup`, `rat-theme4`, `rat-markdown` — 11+ crates. It has 54 stars.

**Rationale:** Discoverability. `use rataframe::prelude::*` and everything is available. No dependency version matrix. No "which sub-crate has the focus manager?" questions. Features not used are dead-code eliminated by the compiler.

---

## ADR-010: Zero Proc Macros for Core API

**Decision:** The `Application` and `Panel` traits are plain Rust. No derive macros, no proc macros.

**Rationale:**
- Proc macros increase compile times
- Proc macros are opaque — errors are hard to debug
- Plain traits are readable, greppable, and well-supported by IDEs
- A convenience `keys![]` macro for keybinding definitions is acceptable but not required

---

## ADR-011: TUI-Safe Logging

**Decision:** The framework automatically configures `tracing` with a file appender. No log output goes to stdout/stderr (which would corrupt the TUI).

**Rationale:** Every TUI developer discovers on day one that `println!()` corrupts the terminal. This is the #1 pain point in TUI development. The framework MUST solve it. Auto-configured file logging means developers can `tail -f app.log` in another terminal during development.

---

## ADR-012: TestHarness Ships in the Crate

**Decision:** A `TestHarness` struct for simulating events and asserting state is part of the framework, not a separate testing crate.

**Rationale:** If testing is hard, people don't test. The harness provides `press_key()`, `send_message()`, `app()`, `is_panel_focused()`, `has_overlay()`. Apps are testable from day one without any external dependencies.

---

## ADR-013: Universal Framework, Not Vortix-Shaped

**Decision:** Design the framework to support thousands of diverse TUI apps, not to extract patterns from one specific app (Vortix).

**Context:** The initial plan was to "extract proven patterns from Vortix into a modular framework." This was too narrow.

**Rationale:** A system monitor (bottom), a text editor (helix), a file manager (yazi), a git UI (gitui), and a simple CLI tool all have different needs. The framework must accommodate all of them:

| Archetype | Panel model | Overlays | Modes | Real-time |
|-----------|------------|----------|-------|-----------|
| Dashboard (btop) | Multi-panel fixed | Few | No | Yes |
| List/Detail (gitui) | Master-detail | Many | No | Some |
| Editor (helix) | Tabs/buffers | Command palette | Yes | No |
| File manager (yazi) | 3-column | Yes | Yes | No |
| Simple tool | Single view | Optional | No | No |

The convention path covers dashboards and list/detail apps. The escape hatch covers editors and exotic layouts.
