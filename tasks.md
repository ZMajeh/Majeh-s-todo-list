# Remaining Tasks Breakdown (from remain.md)

## Task 1: Settings & Subtask Add Button Hover/Static Visibility
- [x] Add a settings state / persistence for static subtask visibility (`static_subtask_visibility: bool`, default `false`).
- [x] Add a checkbox in the **Settings** view to toggle static visibility of subtask add buttons and text fields.
- [x] Update task rendering logic so that when `static_subtask_visibility` is `true`:
  - Hover requirements are disabled, and subtask add buttons/text fields are statically visible.
- [x] When `static_subtask_visibility` is `false`:
  - Retain the current hover-based visibility flow.

## Task 2: Main Task Collapse and Expand
- [x] Ensure main tasks support collapsing and expanding (caret / chevron icon toggle in header) as shown in `AppDesktopView.png`.
- [x] Hide/show subtask tree and children recursively when collapsed/expanded.

## Task 3: Subtask Due Date & Priority Alignment in Columns
- [x] Ensure every subtask displays its **Due Date** and **Priority** (with proper badge/flag).
- [x] Align due date and priority columns properly across main tasks and subtasks so the UI maintains a clean table/list structure matching the desktop design.

## Task 4: "..." (Ellipsis) Delete Menu for Main Tasks and Nested Subtasks
- [x] Add a "..." (ellipsis) button at the end of every main task, subtask, sub-subtask, and nested subtasks.
- [x] Implement an LMB-triggered popup menu (like an RMB menu) when clicking "..." with a **Delete** option.
- [x] Implement recursive deletion logic:
  - Deleting a task deletes all its subtasks and nested descendants.
  - Deleting a subtask deletes all its sub-subtasks and nested descendants.
- [x] Ensure state updates trigger re-renders and persist to `localStorage`.
