# Ultimate Plan: Tree-Based Subtask/Sub-Subtask Widget & UI Refinement

## Background & Objective
Comparing `appCurrentStatus.png` (current implementation) with `SubSubTask_example.png` (target design) reveals significant discrepancies in UI polish, tree hierarchy rendering, priority badge styling, subtask addition flow, and sidebar details. 

The objective of this plan is to refactor the Leptos/Rust frontend (`majeh_todo`) to achieve pixel-parity and robust logic with the target design:
1. **Tree-Based Table Widget & Hierarchy**: Clean collapsible tree nodes with guide lines, proper indentation, and consistent column alignment (Task, Due Date, Priority, Progress).
2. **Subtask & Sub-Subtask Addition Popup**: "+ Add subtask" button triggering an intuitive task/subtask creation popup modal or inline form supporting recursive nesting.
3. **Priority Badges & Styling**: Sleek pill-shaped priority badges (High = red, Medium = amber, Low = green), modern dark/light theme support, and accurate progress indicators.
4. **Sidebar Details Panel**: Full-featured right sidebar displaying task details, nested subtask tree, and quick actions when a task is selected.

---

## Key Files & Context
- `majeh_todo/src/components/task_list.rs`: Core task and subtask recursive rendering, tree node expansion, add/toggle/delete logic, and table layout.
- `majeh_todo/src/pages/dashboard.rs`: Dashboard layout containing sidebar, header, filters, and main task table container.
- `majeh_todo/style.css`: CSS styles for task grid rows, priority badges, custom checkboxes, tree connection lines, and popup modals.

---

## Implementation Steps

### Phase 1: CSS & Styling Alignment (`majeh_todo/style.css`)
- Update priority badge classes (`.priority-badge`, `.priority-high`, `.priority-medium`, `.priority-low`) to match target pill design (rounded corners, subtle background tints, clean text).
- Add tree guide lines (`border-left`, padding, indentation) for nested subtasks and sub-subtasks.
- Ensure grid columns (`.task-grid-row`) align perfectly across main tasks, subtasks, and sub-subtasks.

### Phase 2: Recursive Subtask Data Model & Logic (`majeh_todo/src/components/task_list.rs`)
- Enhance `TodoSubtask` struct to support recursive `subtasks: Vec<TodoSubtask>` with robust serialization.
- Refactor `render_subtasks_recursive_view` to support:
  - Collapse/expand toggle for parent subtasks.
  - Tree branch indicator lines.
  - Clean "+ Add subtask" button opening a modal or inline form popup.
  - Proper recursive add, toggle completion, and delete handlers.

### Phase 3: Add Subtask Popup / Modal & Inline Creation Flow
- Implement a modal dialog component triggered by the "+ Add subtask" / "+ Sub" button for specifying subtask title, priority, due date, and tags.
- Ensure state persistence to `localStorage` (`majeh_tasks`) on every mutation.

### Phase 4: Verification & Testing
- Build and run Trunk/Leptos app (`cd majeh_todo && trunk serve` or `cargo build`) to verify compilation and UI appearance.
- Test adding subtasks and sub-subtasks recursively, toggling completion, checking progress bars, and collapsing/expanding nodes.

---

## Verification Plan
1. Compile successfully without warnings or errors.
2. Verify tree structure renders subtasks and sub-subtasks with proper indentation and tree lines matching `SubSubTask_example.png`.
3. Verify "+ Add subtask" button successfully adds nested sub-subtasks via popup/inline form.
4. Verify priority badges and progress bars match target visual specifications.
