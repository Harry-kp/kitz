# API Reference

The complete API documentation is published on docs.rs and generated directly from the source code:

**[docs.rs/kitz](https://docs.rs/kitz)**

This page provides a quick reference for the most commonly used types.

## Core Types

| Type | Module | Purpose |
|---|---|---|
| `Application` | `kitz::app` | The central trait every kitz app implements. Defines `update`, `view`, `handle_event`, `panels`, and lifecycle hooks. |
| `Command<M>` | `kitz::command` | Describes side-effects returned from `update`. Constructors: `none()`, `quit()`, `message()`, `perform()`, `batch()`. |
| `Context<M>` | `kitz::context` | Mutable context passed to `update`. Used to push overlays, change panel focus, show toasts, and manage the navigation stack. |
| `EventResult<M>` | `kitz::app` | Returned from `handle_event` and `panel_handle_key`. Variants: `Message(M)`, `Ignored`, `Consumed`. |
| `EventContext` | `kitz::context` | Read-only context passed to `handle_event`. Provides `focused_panel()` and `has_overlay()`. |
| `ViewContext` | `kitz::context` | Read-only context passed to `view`. Provides `focused_panel()` and `is_zoomed()`. |

## Panel System

| Type | Module | Purpose |
|---|---|---|
| `PanelLayout` | `kitz::panel` | Describes how panels are arranged. Variants: `None`, `Single`, `Horizontal`, `Vertical`, `Nested`. |
| `PanelId` | `kitz::panel` | Type alias for `&'static str`. Identifies a panel by name. |
| `KeyHint` | `kitz::panel` | A key-description pair shown in the footer and help overlay. Constructor: `KeyHint::new(key, desc)`. |

## Overlays

| Type | Module | Purpose |
|---|---|---|
| `Overlay<M>` | `kitz::overlay` | Trait for modal UI layers rendered on top of the main content. Implement for custom overlays. |
| `ConfirmOverlay` | `kitz::overlay` | Built-in yes/no confirmation dialog. Constructor: `ConfirmOverlay::new(prompt, confirm_msg)`. |
| `HelpOverlay` | `kitz::overlay` | Built-in help screen showing key hints grouped by panel. |
| `CommandPaletteOverlay` | `kitz::overlay` | Built-in fuzzy command palette. |

## Screens

| Type | Module | Purpose |
|---|---|---|
| `Screen<M>` | `kitz::screen` | Trait for full-screen views that can be pushed onto the navigation stack. |

## Subscriptions

| Type | Module | Purpose |
|---|---|---|
| `Subscription<M>` | `kitz::subscription` | Declarative background task managed by the runtime. Constructor: `Subscription::every(id, interval, msg_fn)`. |

## Theming

| Type | Module | Purpose |
|---|---|---|
| `Theme` | `kitz::theme` | Semantic color theme with fields for `bg`, `surface`, `text`, `text_muted`, `border`, `border_focused`, `accent`, `success`, `warning`, `error`. Method: `next()` cycles palettes. |

## Testing

| Type | Module | Purpose |
|---|---|---|
| `TestHarness<A>` | `kitz::testing` | Headless test runner. Methods: `new()`, `press_key()`, `send_key()`, `press_panel_key()`, `send_message()`, `app()`, `app_mut()`, `quit_requested()`. |

## Widgets

| Type | Module | Purpose |
|---|---|---|
| `TextInput` / `TextInputState` | `kitz::widgets` | Stateful text input widget with cursor management. |
| `Footer` | `kitz::widgets` | Renders key hints in a horizontal bar. Used automatically by the panel system. |
| `centered_rect` | `kitz::widgets` | Helper to compute a centered `Rect` within a parent area. |

## Toast Notifications

| Type | Module | Purpose |
|---|---|---|
| `ToastLevel` | `kitz::toast` | Severity level for toasts: `Info`, `Success`, `Warning`, `Error`. |

## Prelude

Import the most common types with a single line:

```rust
use kitz::prelude::*;
```

This re-exports: `Application`, `EventResult`, `Command`, `Context`, `EventContext`, `ViewContext`, `PanelLayout`, `PanelId`, `KeyHint`, `Screen`, `Subscription`, `TestHarness`, `Theme`, `ToastLevel`, `ConfirmOverlay`, `CommandPaletteOverlay`, `PaletteCommand`, `TextInput`, `TextInputState`, `centered_rect`, `Frame`, `Rect`, `Layout`, `Constraint`, `Paragraph`, `Event`, `KeyCode`, `KeyEvent`, `KeyModifiers`, `Result`, and `KitzError`.
