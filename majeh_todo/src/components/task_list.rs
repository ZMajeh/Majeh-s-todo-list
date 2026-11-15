use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    High,
    Medium,
    Low,
}

pub struct RawItem {
    pub depth: usize,
    pub title: String,
    pub number_path: Option<String>,
}

pub fn parse_bulk_tasks(input: &str, due_date: chrono::NaiveDate, priority: Priority, icon: String) -> Result<Vec<TodoTask>, String> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        return Err("Input is empty".to_string());
    }

    let mut raw_items = Vec::new();

    for (idx, raw_line) in lines.iter().enumerate() {
        let trimmed_end = raw_line.trim_end();
        if trimmed_end.is_empty() {
            continue;
        }

        let leading_spaces = raw_line.chars().take_while(|c| c.is_whitespace() || *c == '\t').count();
        let indent_depth = leading_spaces / 2;
        let trimmed = trimmed_end.trim();

        let mut number_path: Option<String> = None;
        let mut number_depth: Option<usize> = None;
        let mut clean_title = trimmed.to_string();

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if let Some(first) = parts.first() {
            let digits_and_dots = first.chars().all(|c| c.is_ascii_digit() || c == '.');
            if digits_and_dots && (first.contains('.') || first.ends_with('.')) {
                let stripped = first.trim_end_matches('.');
                if !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    let segments: Vec<&str> = stripped.split('.').filter(|s| !s.is_empty()).collect();
                    if !segments.is_empty() {
                        number_path = Some(stripped.to_string());
                        number_depth = Some(segments.len() - 1);
                        clean_title = trimmed[first.len()..].trim().to_string();
                    }
                }
            }
        }

        let depth = if let Some(nd) = number_depth { nd } else { indent_depth };
        if clean_title.is_empty() {
            return Err(format!("Syntax error on line {}: task title cannot be empty", idx + 1));
        }

        raw_items.push(RawItem { depth, title: clean_title, number_path });
    }

    if raw_items.is_empty() {
        return Err("No valid tasks found in input".to_string());
    }

    struct TaskNode {
        depth: usize,
        title: String,
        children: Vec<TaskNode>,
    }

    fn parse_nodes(items: &[RawItem], idx: &mut usize, min_depth: usize) -> Result<Vec<TaskNode>, String> {
        let mut nodes = Vec::new();
        let mut expected_max_depth = min_depth;
        while *idx < items.len() {
            let item = &items[*idx];
            if item.depth < min_depth {
                break;
            }
            if nodes.is_empty() {
                if item.depth != min_depth {
                    return Err(format!("Syntax error on line {}: expected depth {}, found depth {}", *idx + 1, min_depth, item.depth));
                }
            } else {
                if item.depth > expected_max_depth + 1 {
                    return Err(format!("Syntax error on line {}: unexpected indentation jump at depth {}", *idx + 1, item.depth));
                }
            }
            let current_depth = item.depth;
            expected_max_depth = current_depth;
            *idx += 1;
            let children = parse_nodes(items, idx, current_depth + 1)?;
            nodes.push(TaskNode {
                depth: current_depth,
                title: item.title.clone(),
                children,
            });
        }
        Ok(nodes)
    }

    let has_numbering = raw_items.iter().any(|item| item.number_path.is_some());
    let all_numbered = raw_items.iter().all(|item| item.number_path.is_some());

    let nodes = if has_numbering {
        if !all_numbered {
            return Err("Syntax error: cannot mix numbering system and plain indentation in the same input".to_string());
        }

        use std::collections::HashMap;
        let mut map: HashMap<String, TaskNode> = HashMap::new();
        let mut root_paths = Vec::new();
        let mut child_entries = Vec::new();

        for item in &raw_items {
            let path = item.number_path.as_ref().unwrap();
            let parts: Vec<&str> = path.split('.').collect();
            if parts.len() == 1 {
                if !root_paths.contains(path) {
                    root_paths.push(path.clone());
                }
            } else {
                child_entries.push(path.clone());
            }
            map.insert(path.clone(), TaskNode {
                depth: item.depth,
                title: item.title.clone(),
                children: Vec::new(),
            });
        }

        child_entries.sort_by(|a, b| b.matches('.').count().cmp(&a.matches('.').count()));

        for path in child_entries {
            let parts: Vec<&str> = path.split('.').collect();
            let parent_path = parts[..parts.len() - 1].join(".");
            if let Some(child_node) = map.remove(&path) {
                if let Some(parent_node) = map.get_mut(&parent_path) {
                    parent_node.children.push(child_node);
                } else {
                    return Err(format!("Syntax error: parent '{}' not found for subtask '{}'", parent_path, path));
                }
            }
        }

        let mut roots = Vec::new();
        for r_path in root_paths {
            if let Some(node) = map.remove(&r_path) {
                roots.push(node);
            }
        }
        roots
    } else {
        let mut idx = 0;
        parse_nodes(&raw_items, &mut idx, 0)?
    };

    fn convert_node_to_subtask(node: TaskNode, priority: Priority, due_date: chrono::NaiveDate) -> TodoSubtask {
        TodoSubtask {
            id: uuid::Uuid::new_v4().to_string(),
            title: node.title,
            completed: false,
            priority: priority.clone(),
            due_date: Some(due_date),
            subtasks: node.children.into_iter().map(|c| convert_node_to_subtask(c, priority.clone(), due_date)).collect(),
        }
    }

    let today = chrono::Local::now().date_naive();
    let tasks: Vec<TodoTask> = nodes.into_iter().map(|node| {
        let subtasks = node.children.into_iter().map(|c| convert_node_to_subtask(c, priority.clone(), due_date)).collect();
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: node.title,
            description: String::new(),
            completed: false,
            subtasks,
            created_date: today,
            due_date: Some(due_date),
            priority: priority.clone(),
            tags: vec!["General".to_string()],
            notes: String::new(),
            icon: icon.clone(),
            collapsed: false,
        }
    }).collect();

    Ok(tasks)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoSubtask {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub priority: Priority,
    #[serde(default)]
    pub due_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    pub subtasks: Vec<TodoSubtask>,
}

pub fn toggle_subtask_recursive(subtasks: &mut Vec<TodoSubtask>, sub_id: &str) -> bool {
    for sub in subtasks.iter_mut() {
        if sub.id == sub_id {
            sub.completed = !sub.completed;
            return true;
        }
        if toggle_subtask_recursive(&mut sub.subtasks, sub_id) {
            return true;
        }
    }
    false
}

pub fn add_subtask_recursive(subtasks: &mut Vec<TodoSubtask>, target_id: &str, new_sub: TodoSubtask) -> bool {
    for sub in subtasks.iter_mut() {
        if sub.id == target_id {
            sub.subtasks.push(new_sub);
            return true;
        }
        if add_subtask_recursive(&mut sub.subtasks, target_id, new_sub.clone()) {
            return true;
        }
    }
    false
}

pub fn add_subtask_to_task(ts: &mut Vec<TodoTask>, task_id: &str, target_id: Option<&str>, new_sub: TodoSubtask) {
    if let Some(t) = ts.iter_mut().find(|x| x.id == task_id) {
        match target_id {
            None => {
                t.subtasks.push(new_sub);
            }
            Some(tid) => {
                add_subtask_recursive(&mut t.subtasks, tid, new_sub);
            }
        }
    }
}

pub fn delete_subtask_recursive(subtasks: &mut Vec<TodoSubtask>, sub_id: &str) -> bool {
    let initial_len = subtasks.len();
    subtasks.retain(|s| s.id != sub_id);
    if subtasks.len() < initial_len {
        return true;
    }
    for sub in subtasks.iter_mut() {
        if delete_subtask_recursive(&mut sub.subtasks, sub_id) {
            return true;
        }
    }
    false
}

pub fn delete_subtask_from_task(ts: &mut Vec<TodoTask>, task_id: &str, sub_id: &str) {
    if let Some(t) = ts.iter_mut().find(|x| x.id == task_id) {
        let initial_len = t.subtasks.len();
        t.subtasks.retain(|s| s.id != sub_id);
        if t.subtasks.len() < initial_len {
            return;
        }
        for sub in t.subtasks.iter_mut() {
            if delete_subtask_recursive(&mut sub.subtasks, sub_id) {
                break;
            }
        }
    }
}

pub fn update_subtask_priority_recursive(subtasks: &mut Vec<TodoSubtask>, sub_id: &str, new_pri: Priority) -> bool {
    for sub in subtasks.iter_mut() {
        if sub.id == sub_id {
            sub.priority = new_pri;
            return true;
        }
        if update_subtask_priority_recursive(&mut sub.subtasks, sub_id, new_pri.clone()) {
            return true;
        }
    }
    false
}

pub fn update_subtask_priority_in_task(ts: &mut Vec<TodoTask>, task_id: &str, sub_id: &str, new_pri: Priority) {
    if let Some(t) = ts.iter_mut().find(|x| x.id == task_id) {
        update_subtask_priority_recursive(&mut t.subtasks, sub_id, new_pri);
    }
}

pub fn update_subtask_due_date_recursive(subtasks: &mut Vec<TodoSubtask>, sub_id: &str, new_due: Option<chrono::NaiveDate>) -> bool {
    for sub in subtasks.iter_mut() {
        if sub.id == sub_id {
            sub.due_date = new_due;
            return true;
        }
        if update_subtask_due_date_recursive(&mut sub.subtasks, sub_id, new_due) {
            return true;
        }
    }
    false
}

pub fn update_subtask_due_date_in_task(ts: &mut Vec<TodoTask>, task_id: &str, sub_id: &str, new_due: Option<chrono::NaiveDate>) {
    if let Some(t) = ts.iter_mut().find(|x| x.id == task_id) {
        update_subtask_due_date_recursive(&mut t.subtasks, sub_id, new_due);
    }
}

pub fn render_subtasks_recursive_view(
    subtasks: Vec<TodoSubtask>,
    task_id: String,
    parent_due: Option<chrono::NaiveDate>,
    toggle_subtask: impl Fn(String, String) + Clone + 'static + Send,
    add_sub_recursive: impl Fn(String, String, String, Priority, Option<chrono::NaiveDate>) + Clone + 'static + Send,
    delete_subtask: impl Fn(String, String) + Clone + 'static + Send,
    update_priority: impl Fn(String, String, Priority) + Clone + 'static + Send,
    update_due_date: impl Fn(String, String, Option<chrono::NaiveDate>) + Clone + 'static + Send,
    static_subtask_visible: ReadSignal<bool>,
    depth: usize,
    compact: bool,
) -> impl IntoView {
    if subtasks.is_empty() {
        return view! { <span /> }.into_any();
    }
    view! {
        <div style="display: flex; flex-direction: column; gap: 1px;">
            {subtasks.into_iter().map(|sub| {
                let sub_id = sub.id.clone();
                let sub_id_add = sub.id.clone();
                let sub_id_del = sub.id.clone();
                let _t_id = task_id.clone();
                let sub_comp = sub.completed;
                let sub_title = sub.title;
                let sub_pri = sub.priority;
                let sub_due = sub.due_date;
                let nested_subs = sub.subtasks;

                let sub_pri_text = match sub_pri {
                    Priority::High => "High",
                    Priority::Medium => "Medium",
                    Priority::Low => "Low",
                };
                let sub_pri_class = match sub_pri {
                    Priority::High => "priority-badge priority-high",
                    Priority::Medium => "priority-badge priority-medium",
                    Priority::Low => "priority-badge priority-low",
                };
                let sub_checkbox_class = if sub_comp { "task-checkbox-custom completed" } else { "task-checkbox-custom" };

                let effective_due = sub_due.or(parent_due);
                let sub_due_str = match effective_due {
                    Some(d) if d == chrono::Local::now().date_naive() => "Today".to_string(),
                    Some(d) => d.format("%b %d, %Y").to_string(),
                    None => "".to_string(),
                };

                let nested_input_val = signal(String::new());
                let nested_pri_val = signal(Priority::Medium);
                let _toggle_cb = toggle_subtask.clone();
                let add_cb = add_sub_recursive.clone();
                let t_id_2 = task_id.clone();

                let add_nested_sub = {
                    let add_cb = add_cb.clone();
                    let t_id_2 = t_id_2.clone();
                    let sub_id_add = sub_id_add.clone();
                    let nested_input_val = nested_input_val.clone();
                    let nested_pri_val = nested_pri_val.clone();
                    move |_| {
                        let title = nested_input_val.0.get();
                        let pri = nested_pri_val.0.get();
                        if !title.is_empty() {
                            add_cb(t_id_2.clone(), sub_id_add.clone(), title, pri, sub_due);
                            nested_input_val.1.set(String::new());
                        }
                    }
                };

                let toggle_cb_2 = toggle_subtask.clone();
                let t_id_3 = task_id.clone();
                let show_sub_menu = signal(false);
                let del_cb = delete_subtask.clone();
                let t_id_del = task_id.clone();

                let sub_completed_count = nested_subs.iter().filter(|s| s.completed).count();
                let sub_total_count = nested_subs.len();
                let sub_progress_pct = if sub_total_count > 0 {
                    (sub_completed_count * 100) / sub_total_count
                } else {
                    0
                };

                let has_children = !nested_subs.is_empty();
                let sub_collapsed = signal(false);

                view! {
                    <div class="subtask-item-container" data-nested=has_children style=format!("--depth: {}; background: var(--bg-hover); border-radius: 4px; padding: 0; margin-top: 1px;", depth)>
                        {if compact {
                            view! {
                                <div class="task-row" style=format!("--depth: {}; display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 4px 6px;", depth)>
                                    <div class="task-cell" style=format!("--depth: {}; display: flex; align-items: center; gap: 8px;", depth)>
                                        {if has_children {
                                            view! {
                                                <button
                                                    style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 11px; padding: 2px;"
                                                    title="Collapse / Expand"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        let cur = sub_collapsed.0.get();
                                                        sub_collapsed.1.set(!cur);
                                                    }
                                                >
                                                    {move || if sub_collapsed.0.get() { "▶" } else { "▼" }}
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! { <span /> }.into_any()
                                        }}
                                        <div
                                            class=sub_checkbox_class
                                            style="width: 16px; height: 16px; font-size: 10px; cursor: pointer;"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                toggle_cb_2(t_id_3.clone(), sub_id.clone());
                                            }
                                        >
                                            {if sub_comp { "✓" } else { "" }}
                                        </div>
                                        <span style="font-size: 14px; margin-right: 2px; flex-shrink: 0;">"📁"</span>
                                        <span style={if sub_comp { "text-decoration: line-through; color: var(--text-muted);" } else { "font-weight: 500; color: var(--text-main);" }}>{sub_title.clone()}</span>
                                    </div>
                                    <div style="display: flex; justify-content: flex-end; position: relative;">
                                        <button
                                            style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px; font-weight: bold; padding: 0 4px;"
                                            title="Subtask Options"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                let current = show_sub_menu.0.get();
                                                show_sub_menu.1.set(!current);
                                            }
                                        >
                                            "⋯"
                                        </button>
                                        {
                                            let del_cb = del_cb.clone();
                                            let t_id_del = t_id_del.clone();
                                            let sub_id_del = sub_id_del.clone();
                                            let show_sub_menu = show_sub_menu.clone();
                                            let up_pri = update_priority.clone();
                                            let up_due = update_due_date.clone();
                                            let tid_p = t_id_del.clone();
                                            let sid_p = sub_id_del.clone();
                                            let tid_d = t_id_del.clone();
                                            let sid_d = sub_id_del.clone();
                                            move || {
                                                let d = del_cb.clone();
                                                let tid = t_id_del.clone();
                                                let sid = sub_id_del.clone();
                                                let sm = show_sub_menu.clone();
                                                let up_p = up_pri.clone();
                                                let up_d = up_due.clone();
                                                let tp = tid_p.clone();
                                                let sp = sid_p.clone();
                                                let td = tid_d.clone();
                                                let sd = sid_d.clone();
                                                if sm.0.get() {
                                                    view! {
                                                        <div style="position: absolute; right: 0; top: 22px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 170px; padding: 6px; font-size: 12px;">
                                                            <div style="padding: 4px 8px; font-weight: 600; color: var(--text-muted); border-bottom: 1px solid var(--border-color); margin-bottom: 4px;">"Priority"</div>
                                                            <button style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; border-radius: 4px;" on:click={
                                                                let sm = sm.clone();
                                                                let up_p = up_p.clone();
                                                                let tp = tp.clone();
                                                                let sp = sp.clone();
                                                                move |_| {
                                                                    sm.1.set(false);
                                                                    up_p(tp.clone(), sp.clone(), Priority::High);
                                                                }
                                                            }>"🔴 High"</button>
                                                            <button style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; border-radius: 4px;" on:click={
                                                                let sm = sm.clone();
                                                                let up_p = up_p.clone();
                                                                let tp = tp.clone();
                                                                let sp = sp.clone();
                                                                move |_| {
                                                                    sm.1.set(false);
                                                                    up_p(tp.clone(), sp.clone(), Priority::Medium);
                                                                }
                                                            }>"🟡 Medium"</button>
                                                            <button style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; border-radius: 4px;" on:click={
                                                                let sm = sm.clone();
                                                                let up_p = up_p.clone();
                                                                let tp = tp.clone();
                                                                let sp = sp.clone();
                                                                move |_| {
                                                                    sm.1.set(false);
                                                                    up_p(tp.clone(), sp.clone(), Priority::Low);
                                                                }
                                                            }>"🟢 Low"</button>
                                                            <div style="padding: 4px 8px; font-weight: 600; color: var(--text-muted); border-bottom: 1px solid var(--border-color); border-top: 1px solid var(--border-color); margin: 4px 0;">"Due Date"</div>
                                                            <button style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; border-radius: 4px;" on:click={
                                                                let sm = sm.clone();
                                                                let up_d = up_d.clone();
                                                                let td = td.clone();
                                                                let sd = sd.clone();
                                                                move |_| {
                                                                    sm.1.set(false);
                                                                    up_d(td.clone(), sd.clone(), Some(chrono::Local::now().date_naive()));
                                                                }
                                                            }>"Set Due: Today"</button>
                                                            <button style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; border-radius: 4px;" on:click={
                                                                let sm = sm.clone();
                                                                let up_d = up_d.clone();
                                                                let td = td.clone();
                                                                let sd = sd.clone();
                                                                move |_| {
                                                                    sm.1.set(false);
                                                                    up_d(td.clone(), sd.clone(), None);
                                                                }
                                                            }>"Clear Due Date"</button>
                                                            <div style="border-top: 1px solid var(--border-color); margin-top: 4px; padding-top: 4px;">
                                                                <button
                                                                    style="background: transparent; border: none; padding: 4px 8px; text-align: left; cursor: pointer; color: #dc2626; width: 100%; border-radius: 4px;"
                                                                    on:click={
                                                                        let sm = sm.clone();
                                                                        let d = d.clone();
                                                                        let tid = tid.clone();
                                                                        let sid = sid.clone();
                                                                        move |_| {
                                                                            sm.1.set(false);
                                                                            d(tid.clone(), sid.clone());
                                                                        }
                                                                    }
                                                                >
                                                                    "Delete Subtask"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }
                                            }
                                        }
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="task-row" data-nested=has_children style=format!("--depth: {};", depth)>
                                    <div class="task-cell" style=format!("--depth: {};", depth)>
                                        {if has_children {
                                            view! {
                                                <button
                                                    style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 11px; padding: 2px;"
                                                    title="Collapse / Expand"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        let cur = sub_collapsed.0.get();
                                                        sub_collapsed.1.set(!cur);
                                                    }
                                                >
                                                    {move || if sub_collapsed.0.get() { "▶" } else { "▼" }}
                                                </button>
                                                <div
                                                    class=sub_checkbox_class
                                                    style="width: 16px; height: 16px; font-size: 10px; cursor: pointer;"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        toggle_cb_2(t_id_3.clone(), sub_id.clone());
                                                    }
                                                >
                                                    {if sub_comp { "✓" } else { "" }}
                                                </div>
                                                <span style="font-size: 14px; margin-right: 2px; flex-shrink: 0;">"📁"</span>
                                                <span style={if sub_comp { "text-decoration: line-through; color: var(--text-muted);" } else { "font-weight: 600; color: var(--text-main);" }}>{sub_title.clone()}</span>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div
                                                    class=sub_checkbox_class
                                                    style="width: 16px; height: 16px; font-size: 10px; cursor: pointer;"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        toggle_cb_2(t_id_3.clone(), sub_id.clone());
                                                    }
                                                >
                                                    {if sub_comp { "✓" } else { "" }}
                                                </div>
                                                <span style={if sub_comp { "text-decoration: line-through; color: var(--text-muted);" } else { "" }}>
                                                    {sub_title.clone()}
                                                </span>
                                            }.into_any()
                                        }}
                                    </div>
                                    <span style="font-size: 13px; color: var(--text-muted);">{sub_due_str}</span>
                                    <div><span class=sub_pri_class style="font-size: 11px; padding: 2px 8px;">{sub_pri_text}</span></div>
                                    <div style="display: flex; flex-direction: column; align-items: flex-end; gap: 4px;">
                                        {if sub_total_count > 0 {
                                            view! {
                                                <span style="font-size: 11px; color: var(--text-muted);">{format!("{}/{}", sub_completed_count, sub_total_count)}</span>
                                                <div style="width: 70px; background: var(--border-color); height: 4px; border-radius: 2px; overflow: hidden;">
                                                    <div style=format!("background: var(--primary); width: {}%; height: 100%;", sub_progress_pct)></div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span /> }.into_any()
                                        }}
                                    </div>
                                    <div style="display: flex; justify-content: flex-end; position: relative;">
                                        <button
                                            style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px; font-weight: bold;"
                                            title="Subtask Options"
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                let current = show_sub_menu.0.get();
                                                show_sub_menu.1.set(!current);
                                            }
                                        >
                                            "⋯"
                                        </button>
                                        {
                                            let del_cb = del_cb.clone();
                                            let t_id_del = t_id_del.clone();
                                            let sub_id_del = sub_id_del.clone();
                                            let show_sub_menu = show_sub_menu.clone();
                                            move || {
                                                let d = del_cb.clone();
                                                let tid = t_id_del.clone();
                                                let sid = sub_id_del.clone();
                                                let sm = show_sub_menu.clone();
                                                if sm.0.get() {
                                                    view! {
                                                        <div style="position: absolute; right: 0; top: 22px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 100px; padding: 4px;">
                                                            <button
                                                                style="background: transparent; border: none; padding: 6px 10px; text-align: left; cursor: pointer; color: #dc2626; font-size: 12px; border-radius: 4px;"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    sm.1.set(false);
                                                                    d(tid.clone(), sid.clone());
                                                                }
                                                            >
                                                                "Delete"
                                                            </button>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }
                                            }
                                        }
                                    </div>
                                </div>
                            }.into_any()
                        }}
                        {
                            let task_id_rec = task_id.clone();
                            let toggle_sub_rec = toggle_subtask.clone();
                            let add_sub_rec = add_sub_recursive.clone();
                            let delete_sub_rec = delete_subtask.clone();
                            let update_pri_rec = update_priority.clone();
                            let update_due_rec = update_due_date.clone();
                            let comp = compact;
                            move || {
                                if !has_children || !sub_collapsed.0.get() {
                                    view! {
                                        <div>
                                            {
                                                let cls = move || if static_subtask_visible.get() { "subtask-add-bar static-visible" } else { "subtask-add-bar" };
                                                let sty = format!("display: flex; gap: 6px; margin-top: 4px; margin-left: calc({} * 14px + 14px);", depth + 1);
                                                view! {
                                                    <div class=cls style=sty on:click=move |ev| ev.stop_propagation()>
                                                        <input
                                                            type="text"
                                                            placeholder="Add sub-subtask..."
                                                            on:input=move |ev| nested_input_val.1.set(event_target_value(&ev))
                                                            prop:value=nested_input_val.0
                                                            class="task-input"
                                                            style="font-size: 11px; padding: 4px 8px; max-width: 160px;"
                                                        />
                                                        <select
                                                            style="background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 4px; padding: 2px; font-size: 10px;"
                                                            on:change=move |ev| {
                                                                let val = event_target_value(&ev);
                                                                let p = match val.as_str() {
                                                                    "High" => Priority::High,
                                                                    "Low" => Priority::Low,
                                                                    _ => Priority::Medium,
                                                                };
                                                                nested_pri_val.1.set(p);
                                                            }
                                                        >
                                                            <option value="Medium">"Med"</option>
                                                            <option value="High">"High"</option>
                                                            <option value="Low">"Low"</option>
                                                        </select>
                                                        {
                                                            let add_nested_sub_click = add_nested_sub.clone();
                                                            view! {
                                                                <button
                                                                    on:click=add_nested_sub_click
                                                                    style="background: var(--primary); color: white; border: none; padding: 2px 8px; border-radius: 4px; cursor: pointer; font-size: 11px;"
                                                                >
                                                                    "+ Sub"
                                                                </button>
                                                            }
                                                        }
                                                    </div>
                                                }
                                            }
                                            {if !nested_subs.is_empty() {
                                                let tid = task_id_rec.clone();
                                                let eff_due = effective_due;
                                                let t_sub = toggle_sub_rec.clone();
                                                let add_rec = add_sub_rec.clone();
                                                let del_sub = delete_sub_rec.clone();
                                                let up_pri = update_pri_rec.clone();
                                                let up_due = update_due_rec.clone();
                                                let stat_vis = static_subtask_visible;
                                                render_subtasks_recursive_view(nested_subs.clone(), tid, eff_due, t_sub, add_rec, del_sub, up_pri, up_due, stat_vis, depth + 1, comp).into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span /> }.into_any()
                            }
                        }}
                    </div>
                }
            }).collect_view().into_any()}
        </div>
    }.into_any()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub subtasks: Vec<TodoSubtask>,
    pub created_date: chrono::NaiveDate,
    pub due_date: Option<chrono::NaiveDate>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub notes: String,
    pub icon: String,
    #[serde(default)]
    pub collapsed: bool,
}

pub fn load_or_default_tasks() -> Vec<TodoTask> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("majeh_tasks") {
                if let Ok(tasks) = serde_json::from_str::<Vec<TodoTask>>(&json) {
                    if !tasks.is_empty() {
                        return tasks;
                    }
                }
            }
        }
    }
    let today = chrono::Local::now().date_naive();
    vec![
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Build Website".to_string(),
            description: "A complete website with modern design and authentication".to_string(),
            completed: false,
            subtasks: vec![
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Design Homepage".to_string(), completed: true, priority: Priority::Medium, due_date: Some(today), subtasks: vec![] },
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Implement Authentication".to_string(), completed: true, priority: Priority::High, due_date: Some(today), subtasks: vec![] },
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Build Dashboard UI".to_string(), completed: false, priority: Priority::Medium, due_date: Some(today), subtasks: vec![] },
            ],
            created_date: today,
            due_date: Some(today),
            priority: Priority::High,
            tags: vec!["Website".to_string(), "Development".to_string()],
            notes: "Build a modern, responsive website with authentication and todo functionality.".to_string(),
            icon: "💻".to_string(),
            collapsed: false,
        },
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Write Project Documentation".to_string(),
            description: "Technical and user documentation".to_string(),
            completed: false,
            subtasks: vec![],
            created_date: today,
            due_date: Some(today),
            priority: Priority::Medium,
            tags: vec!["Docs".to_string()],
            notes: "".to_string(),
            icon: "📄".to_string(),
            collapsed: false,
        },
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Plan Vacation Trip".to_string(),
            description: "Family vacation to Japan".to_string(),
            completed: false,
            subtasks: vec![
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Research Destinations".to_string(), completed: true, priority: Priority::Low, due_date: Some(today + chrono::Duration::days(6)), subtasks: vec![] },
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Book Flights".to_string(), completed: false, priority: Priority::Medium, due_date: Some(today + chrono::Duration::days(6)), subtasks: vec![] },
                TodoSubtask { id: uuid::Uuid::new_v4().to_string(), title: "Book Hotels".to_string(), completed: false, priority: Priority::Medium, due_date: Some(today + chrono::Duration::days(7)), subtasks: vec![] },
            ],
            created_date: today,
            due_date: Some(today + chrono::Duration::days(6)),
            priority: Priority::Low,
            tags: vec!["Personal".to_string(), "Travel".to_string()],
            notes: "Family trip planning to Tokyo and Kyoto.".to_string(),
            icon: "✈️".to_string(),
            collapsed: false,
        },
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Setup Sync Engine".to_string(),
            description: "Google Drive integration and offline support".to_string(),
            completed: false,
            subtasks: vec![],
            created_date: today,
            due_date: Some(today + chrono::Duration::days(1)),
            priority: Priority::High,
            tags: vec!["Backend".to_string(), "Sync".to_string()],
            notes: "".to_string(),
            icon: "⚙️".to_string(),
            collapsed: false,
        },
        TodoTask {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Learn Rust & Leptos".to_string(),
            description: "Build expertise in Rust and Leptos framework".to_string(),
            completed: false,
            subtasks: vec![],
            created_date: today,
            due_date: Some(today + chrono::Duration::days(17)),
            priority: Priority::Medium,
            tags: vec!["Learning".to_string(), "Rust".to_string()],
            notes: "".to_string(),
            icon: "📖".to_string(),
            collapsed: false,
        },
    ]
}

#[component]
pub fn TaskList(
    tasks: ReadSignal<Vec<TodoTask>>,
    set_tasks: WriteSignal<Vec<TodoTask>>,
    selected_task: ReadSignal<Option<TodoTask>>,
    set_selected_task: WriteSignal<Option<TodoTask>>,
    selected_date: ReadSignal<chrono::NaiveDate>,
    search_query: ReadSignal<String>,
    active_tab: ReadSignal<String>,
    priority_filter: ReadSignal<String>,
    tag_filter: ReadSignal<String>,
    due_filter: ReadSignal<String>,
    static_subtask_visible: ReadSignal<bool>,
) -> impl IntoView {
    let new_task_title = signal(String::new());
    let new_task_priority = signal(Priority::Medium);
    let new_task_icon = signal("📝".to_string());
    let show_add_modal = signal(false);
    let is_bulk = signal(false);
    let bulk_text = signal(String::new());
    let bulk_error = signal(String::new());

    let add_task = move |_| {
        let pri = new_task_priority.0.get();
        let icn = new_task_icon.0.get();
        if is_bulk.0.get() {
            let text = bulk_text.0.get();
            match parse_bulk_tasks(&text, selected_date.get(), pri, icn) {
                Ok(parsed_tasks) => {
                    set_tasks.update(|ts| {
                        for t in parsed_tasks {
                            ts.push(t.clone());
                        }
                        save_to_local_storage(ts);
                    });
                    bulk_text.1.set(String::new());
                    bulk_error.1.set(String::new());
                    is_bulk.1.set(false);
                    new_task_title.1.set(String::new());
                    show_add_modal.1.set(false);
                }
                Err(err_msg) => {
                    bulk_error.1.set(err_msg);
                }
            }
        } else {
            let title = new_task_title.0.get();
            if !title.is_empty() {
                let new_t = TodoTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title,
                    description: String::new(),
                    completed: false,
                    subtasks: vec![],
                    created_date: chrono::Local::now().date_naive(),
                    due_date: Some(selected_date.get()),
                    priority: pri,
                    tags: vec!["General".to_string()],
                    notes: String::new(),
                    icon: icn,
                    collapsed: false,
                };
                set_tasks.update(|ts| {
                    ts.push(new_t.clone());
                    save_to_local_storage(ts);
                });
                set_selected_task.set(Some(new_t));
                new_task_title.1.set(String::new());
                show_add_modal.1.set(false);
            }
        }
    };

    let toggle_task = move |id: String| {
        set_tasks.update(|ts| {
            if let Some(t) = ts.iter_mut().find(|x| x.id == id) {
                t.completed = !t.completed;
            }
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == id {
                let current_tasks = tasks.get();
                if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                    set_selected_task.set(Some(updated.clone()));
                }
            }
        }
    };

    let toggle_collapse = move |id: String| {
        set_tasks.update(|ts| {
            if let Some(t) = ts.iter_mut().find(|x| x.id == id) {
                t.collapsed = !t.collapsed;
            }
            save_to_local_storage(ts);
        });
    };

    let toggle_subtask = move |parent_id: String, sub_id: String| {
        set_tasks.update(|ts| {
            if let Some(t) = ts.iter_mut().find(|x| x.id == parent_id) {
                toggle_subtask_recursive(&mut t.subtasks, &sub_id);
            }
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == parent_id {
                let current_tasks = tasks.get();
                if let Some(updated) = current_tasks.iter().find(|x| x.id == parent_id) {
                    set_selected_task.set(Some(updated.clone()));
                }
            }
        }
    };

    let delete_task = move |id: String| {
        set_tasks.update(|ts| {
            ts.retain(|x| x.id != id);
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == id {
                set_selected_task.set(None);
            }
        }
    };

    let delete_subtask = move |task_id: String, sub_id: String| {
        set_tasks.update(|ts| {
            delete_subtask_from_task(ts, &task_id, &sub_id);
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == task_id {
                let current_tasks = tasks.get();
                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id) {
                    set_selected_task.set(Some(updated.clone()));
                }
            }
        }
    };

    let update_subtask_priority = move |task_id: String, sub_id: String, new_pri: Priority| {
        set_tasks.update(|ts| {
            update_subtask_priority_in_task(ts, &task_id, &sub_id, new_pri);
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == task_id {
                let current_tasks = tasks.get();
                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id) {
                    set_selected_task.set(Some(updated.clone()));
                }
            }
        }
    };

    let update_subtask_due_date = move |task_id: String, sub_id: String, new_due: Option<chrono::NaiveDate>| {
        set_tasks.update(|ts| {
            update_subtask_due_date_in_task(ts, &task_id, &sub_id, new_due);
            save_to_local_storage(ts);
        });
        if let Some(sel) = selected_task.get() {
            if sel.id == task_id {
                let current_tasks = tasks.get();
                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id) {
                    set_selected_task.set(Some(updated.clone()));
                }
            }
        }
    };

    let select_task = move |t: TodoTask| {
        set_selected_task.set(Some(t));
    };

    view! {
        <section class="task-list-section">
            <div class="task-table-scroll-container">
                // Task Table Header matching plan
                <div class="task-table-header">
                    <span>"Task"</span>
                    <span>"Due Date"</span>
                    <span>"Priority"</span>
                    <span style="text-align: right;">"Progress"</span>
                    <span />
                </div>

                <div style="display: flex; flex-direction: column; gap: 12px;">
                    {move || {
                        let list = tasks.get();
                        let query = search_query.get().to_lowercase();
                        let tab = active_tab.get();
                        let date = selected_date.get();

                    let filtered: Vec<TodoTask> = list.into_iter().filter(|task| {
                        let matches_tab = match tab.as_str() {
                            "active" => !task.completed,
                            "completed" => task.completed,
                            _ => true,
                        };

                        let matches_query = query.is_empty() 
                            || task.title.to_lowercase().contains(&query)
                            || task.description.to_lowercase().contains(&query)
                            || task.notes.to_lowercase().contains(&query)
                            || task.tags.iter().any(|tag| tag.to_lowercase().contains(&query));

                        let matches_date = match task.due_date {
                            Some(due) => due <= date,
                            None => true,
                        };

                        let pri_f = priority_filter.get();
                        let matches_priority = match pri_f.as_str() {
                            "High" => task.priority == Priority::High,
                            "Medium" => task.priority == Priority::Medium,
                            "Low" => task.priority == Priority::Low,
                            _ => true,
                        };

                        let tag_f = tag_filter.get();
                        let matches_tag = match tag_f.as_str() {
                            "All Tags" => true,
                            t => task.tags.iter().any(|tag| tag == t),
                        };

                        let due_f = due_filter.get();
                        let matches_due_filter = match due_f.as_str() {
                            "Today" => task.due_date == Some(chrono::Local::now().date_naive()),
                            "Upcoming" => task.due_date.map_or(false, |d| d > chrono::Local::now().date_naive()),
                            _ => true,
                        };

                        matches_tab && matches_query && matches_date && matches_priority && matches_tag && matches_due_filter
                    }).collect();

                    if filtered.is_empty() {
                        view! { <p style="color: var(--text-muted); text-align: center; padding: 20px;">"No tasks found for this view or date."</p> }.into_any()
                    } else {
                        filtered.into_iter().map(|task| {
                            let t_id_toggle = task.id.clone();
                            let t_id_del = task.id.clone();
                            let t_id_collapse = task.id.clone();
                            let task_for_select = task.clone();
                            let is_comp = task.completed;
                            let is_collapsed = task.collapsed;
                            let title = task.title.clone();
                            let desc = task.description.clone();
                            let priority = task.priority.clone();
                            let subtasks = task.subtasks.clone();
                            let parent_id = task.id.clone();
                            let icon = task.icon.clone();
                            let due_date = task.due_date;

                            let priority_class = match priority {
                                Priority::High => "priority-badge priority-high",
                                Priority::Medium => "priority-badge priority-medium",
                                Priority::Low => "priority-badge priority-low",
                            };

                            let priority_text = match priority {
                                Priority::High => "⚑ High",
                                Priority::Medium => "⚑ Medium",
                                Priority::Low => "⚑ Low",
                            };

                            let completed_class = if is_comp { "task-title completed" } else { "task-title" };
                            let completed_count = subtasks.iter().filter(|s| s.completed).count();
                            let total_count = subtasks.len();
                            let progress_pct = if total_count > 0 {
                                (completed_count * 100) / total_count
                            } else if is_comp {
                                100
                            } else {
                                0
                            };
                            let checkbox_class = if is_comp { "task-checkbox-custom completed" } else { "task-checkbox-custom" };

                            let due_str = match due_date {
                                Some(d) if d == chrono::Local::now().date_naive() => "Today".to_string(),
                                Some(d) => d.format("%b %d, %Y").to_string(),
                                None => "No due date".to_string(),
                            };

                            let new_sub_input = signal(String::new());
                            let new_sub_pri_inline = signal(Priority::Medium);
                            let t_id_for_add = parent_id.clone();
                            let add_subtask_inline = move |_| {
                                let sub_title = new_sub_input.0.get();
                                let sub_pri = new_sub_pri_inline.0.get();
                                let pid = t_id_for_add.clone();
                                if !sub_title.is_empty() {
                                    set_tasks.update(|ts| {
                                        let new_sub = TodoSubtask {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            title: sub_title,
                                            completed: false,
                                            priority: sub_pri,
                                            due_date: due_date,
                                            subtasks: vec![],
                                        };
                                        add_subtask_to_task(ts, &pid, None, new_sub);
                                        save_to_local_storage(ts);
                                    });
                                    new_sub_input.1.set(String::new());
                                }
                            };

                            let toggle_subtask_cb = toggle_subtask.clone();
                            let add_sub_recursive_cb = move |tid: String, target_id: String, title: String, pri: Priority, parent_due: Option<chrono::NaiveDate>| {
                                set_tasks.update(|ts| {
                                    let new_sub = TodoSubtask {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        title,
                                        completed: false,
                                        priority: pri,
                                        due_date: parent_due,
                                        subtasks: vec![],
                                    };
                                    add_subtask_to_task(ts, &tid, Some(&target_id), new_sub);
                                    save_to_local_storage(ts);
                                });
                            };

                            let delete_sub_cb = delete_subtask.clone();
                            let show_task_menu = signal(false);

                            view! {
                                <div class="task-item" on:click=move |_| select_task(task_for_select.clone())>
                                    <div class="task-header-row task-grid-row">
                                        <div style="display: flex; flex-direction: column; gap: 4px; min-width: 0;">
                                            <div class="task-title-area">
                                                <button
                                                    style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 11px; padding: 2px;"
                                                    title="Collapse / Expand"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        toggle_collapse(t_id_collapse.clone());
                                                    }
                                                >
                                                    {if is_collapsed { "▶" } else { "▼" }}
                                                </button>
                                                <div
                                                    class=checkbox_class
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        toggle_task(t_id_toggle.clone());
                                                    }
                                                >
                                                    {if is_comp { "✓" } else { "" }}
                                                </div>
                                                <span style="font-size: 16px; margin-right: 4px; flex-shrink: 0;">{icon}</span>
                                                <span class=completed_class style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{title}</span>
                                            </div>
                                            {if !desc.is_empty() {
                                                view! { <div style="font-size: 12px; color: var(--text-muted); margin-left: 52px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{desc.clone()}</div> }.into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }}
                                        </div>
                                        <span style="font-size: 13px; color: var(--text-muted);">{due_str}</span>
                                        <div><span class=priority_class>{priority_text}</span></div>
                                        <div style="display: flex; flex-direction: column; align-items: flex-end; gap: 4px;">
                                            <span style="font-size: 12px; color: var(--text-muted);">{format!("{}/{}", completed_count, total_count)}</span>
                                            <div style="width: 70px; background: var(--border-color); height: 5px; border-radius: 3px; overflow: hidden;">
                                                <div style=format!("background: var(--primary); width: {}%; height: 100%;", progress_pct)></div>
                                            </div>
                                        </div>
                                        <div style="display: flex; justify-content: flex-end; position: relative;">
                                            <button
                                                style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px; font-weight: bold;"
                                                title="Task Options / Delete"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let current = show_task_menu.0.get();
                                                    show_task_menu.1.set(!current);
                                                }
                                            >
                                                "⋯"
                                            </button>
                                            {
                                                let delete_task = delete_task.clone();
                                                let t_id_del = t_id_del.clone();
                                                let show_task_menu = show_task_menu.clone();
                                                move || {
                                                    let dt = delete_task.clone();
                                                    let tid = t_id_del.clone();
                                                    let stm = show_task_menu.clone();
                                                    if stm.0.get() {
                                                        view! {
                                                            <div style="position: absolute; right: 0; top: 24px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 6px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 110px; padding: 4px;">
                                                                <button
                                                                    style="background: transparent; border: none; padding: 6px 10px; text-align: left; cursor: pointer; color: #dc2626; font-size: 12px; border-radius: 4px;"
                                                                    on:click=move |ev| {
                                                                        ev.stop_propagation();
                                                                        stm.1.set(false);
                                                                        dt(tid.clone());
                                                                    }
                                                                >
                                                                    "Delete"
                                                                </button>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span /> }.into_any()
                                                    }
                                                }
                                            }
                                        </div>
                                    </div>
                                    {if !is_collapsed {
                                        view! {
                                            <div>
                                                {if !desc.is_empty() {
                                                    view! { <p style="font-size: 13px; color: var(--text-muted); margin-left: 32px;">{desc}</p> }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }}
                                                {if !subtasks.is_empty() || true {
                                                    view! {
                                                        <div class="subtasks-list">
                                                            {
                                                                let update_pri_cb = update_subtask_priority.clone();
                                                                let update_due_cb = update_subtask_due_date.clone();
                                                                render_subtasks_recursive_view(subtasks.clone(), parent_id.clone(), due_date, toggle_subtask_cb, add_sub_recursive_cb, delete_sub_cb, update_pri_cb, update_due_cb, static_subtask_visible, 1, false)
                                                            }

                                                            {
                                                                let cls = move || if static_subtask_visible.get() { "task-add-subtask-bar static-visible" } else { "task-add-subtask-bar" };
                                                                let sty = "display: flex; gap: 8px; margin-top: 8px; margin-left: 24px;";
                                                                view! {
                                                                    <div class=cls style=sty on:click=move |ev| ev.stop_propagation()>
                                                                        <input
                                                                            type="text"
                                                                            placeholder="Add subtask..."
                                                                            on:input=move |ev| new_sub_input.1.set(event_target_value(&ev))
                                                                            prop:value=new_sub_input.0
                                                                            class="task-input"
                                                                            style="font-size: 12px; padding: 6px 10px; max-width: 200px;"
                                                                        />
                                                                        <select
                                                                            style="background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 6px; padding: 4px; font-size: 11px;"
                                                                            on:change=move |ev| {
                                                                                let val = event_target_value(&ev);
                                                                                let p = match val.as_str() {
                                                                                    "High" => Priority::High,
                                                                                    "Low" => Priority::Low,
                                                                                    _ => Priority::Medium,
                                                                                };
                                                                                new_sub_pri_inline.1.set(p);
                                                                            }
                                                                        >
                                                                            <option value="Medium">"Med"</option>
                                                                            <option value="High">"High"</option>
                                                                            <option value="Low">"Low"</option>
                                                                        </select>
                                                                        <button
                                                                            on:click=add_subtask_inline
                                                                            style="background: var(--primary); color: white; border: none; padding: 0 10px; border-radius: 6px; cursor: pointer; font-size: 12px;"
                                                                        >
                                                                            "+ Add subtask"
                                                                        </button>
                                                                    </div>
                                                                }
                                                            }
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span /> }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <span /> }.into_any()
                                    }}
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>
            </div>

            // + Add Task dashed card at the bottom matching plan
            <div
                style="background: var(--bg-card); border: 1px dashed var(--border-color); padding: 16px; border-radius: var(--radius); text-align: center; color: var(--primary); font-weight: 600; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 8px; transition: all 0.2s; box-shadow: var(--shadow);"
                on:click=move |_| show_add_modal.1.set(true)
            >
                <span style="font-size: 18px;">"+"</span>
                <span>"Add Task"</span>
            </div>

            // Add Task Modal
            {move || {
                if show_add_modal.0.get() {
                    view! {
                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 400px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <h3 style="font-size: 16px; font-weight: 700;">"Add New Task"</h3>
                                    <button class="close-btn" on:click=move |_| show_add_modal.1.set(false)>"×</"</button>
                                </div>
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <input
                                        type="checkbox"
                                        id="bulk-tasks-checkbox"
                                        prop:checked=is_bulk.0
                                        on:change=move |ev| {
                                            let checked = ev.target()
                                                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                                .map(|input| input.checked())
                                                .unwrap_or(false);
                                            is_bulk.1.set(checked);
                                            bulk_error.1.set(String::new());
                                        }
                                    />
                                    <label for="bulk-tasks-checkbox" style="font-size: 13px; font-weight: 500; cursor: pointer;">"Bulk Tasks Add (Multi-line / 1.1 syntax)"</label>
                                </div>

                                {move || {
                                    if is_bulk.0.get() {
                                        view! {
                                            <div style="display: flex; flex-direction: column; gap: 6px;">
                                                <textarea
                                                    placeholder="1. Main Task&#10;  1.1 Sub Task&#10;    1.1.1 Sub-sub Task&#10;or use indentation spaces..."
                                                    class="task-input"
                                                    style="padding: 12px; font-size: 13px; height: 140px; resize: vertical; font-family: monospace;"
                                                    prop:value=bulk_text.0
                                                    on:input=move |ev| {
                                                        bulk_text.1.set(event_target_value(&ev));
                                                        bulk_error.1.set(String::new());
                                                    }
                                                ></textarea>
                                                <span style="font-size: 11px; color: var(--text-muted);">"Each line is a task. Use leading spaces or 1., 1.1, 1.1.1 numbering for nesting."</span>
                                                {move || {
                                                    let err = bulk_error.0.get();
                                                    if !err.is_empty() {
                                                        view! { <span style="font-size: 12px; color: #dc2626; font-weight: 600;">{err}</span> }.into_any()
                                                    } else {
                                                        view! { <span /> }.into_any()
                                                    }
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <input
                                                id="main-task-input"
                                                type="text"
                                                placeholder="Task title..."
                                                class="task-input"
                                                style="padding: 12px; font-size: 14px;"
                                                prop:value=new_task_title.0
                                                on:input=move |ev| new_task_title.1.set(event_target_value(&ev))
                                            />
                                        }.into_any()
                                    }
                                }}
                                <div style="display: flex; gap: 12px;">
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 6px;">
                                        <label style="font-size: 12px; color: var(--text-muted);">"Priority"</label>
                                        <select
                                            style="background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 6px; padding: 8px; font-size: 13px;"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                let p = match val.as_str() {
                                                    "High" => Priority::High,
                                                    "Low" => Priority::Low,
                                                    _ => Priority::Medium,
                                                };
                                                new_task_priority.1.set(p);
                                            }
                                        >
                                            <option value="Medium">"Medium"</option>
                                            <option value="High">"High"</option>
                                            <option value="Low">"Low"</option>
                                        </select>
                                    </div>
                                    <div style="flex: 1; display: flex; flex-direction: column; gap: 6px;">
                                        <label style="font-size: 12px; color: var(--text-muted);">"Icon"</label>
                                        <select
                                            style="background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 6px; padding: 8px; font-size: 13px;"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                new_task_icon.1.set(val);
                                            }
                                        >
                                            <option value="📝">"📝 Note"</option>
                                            <option value="💻">"💻 Code"</option>
                                            <option value="📄">"📄 Doc"</option>
                                            <option value="✈️">"✈️ Travel"</option>
                                            <option value="⚙️">"⚙️ Settings"</option>
                                            <option value="📖">"📖 Read"</option>
                                            <option value="🚀">"🚀 Launch"</option>
                                            <option value="💡">"💡 Idea"</option>
                                        </select>
                                    </div>
                                </div>
                                <div style="display: flex; gap: 10px; justify-content: flex-end;">
                                    <button style="background: transparent; border: 1px solid var(--border-color); padding: 8px 16px; border-radius: 8px; cursor: pointer;" on:click=move |_| show_add_modal.1.set(false)>"Cancel"</button>
                                    <button class="add-task-btn" style="padding: 8px 16px;" on:click=add_task>"Create Task"</button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}
        </section>
    }
}

pub fn save_to_local_storage(tasks: &[TodoTask]) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(tasks) {
                let _ = storage.set_item("majeh_tasks", &json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bulk_tasks_basic() {
        let input = "Task 1\nTask 2";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let res = parse_bulk_tasks(input, today, Priority::Medium, "📝".to_string());
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Task 1");
        assert_eq!(tasks[1].title, "Task 2");
    }

    #[test]
    fn test_parse_bulk_tasks_indentation() {
        let input = "Main Task\n  Subtask 1\n    Subsubtask 1";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let res = parse_bulk_tasks(input, today, Priority::High, "🚀".to_string());
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Main Task");
        assert_eq!(tasks[0].subtasks.len(), 1);
        assert_eq!(tasks[0].subtasks[0].title, "Subtask 1");
        assert_eq!(tasks[0].subtasks[0].subtasks.len(), 1);
        assert_eq!(tasks[0].subtasks[0].subtasks[0].title, "Subsubtask 1");
    }

    #[test]
    fn test_parse_bulk_tasks_multilevel_numbering() {
        let input = "1. one task\n2. second task\n1.1 sub for 1\n2.1 sub for 2\n1.2 sub for 1 again\n2.2 sub for 2 again\n1.1.1 sub of one's sub\n2.1.1 sub of 2's one";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let res = parse_bulk_tasks(input, today, Priority::Medium, "📝".to_string());
        assert!(res.is_ok());
        let tasks = res.unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "one task");
        assert_eq!(tasks[0].subtasks.len(), 2); // 1.1 and 1.2
        assert_eq!(tasks[0].subtasks[0].title, "sub for 1");
        assert_eq!(tasks[0].subtasks[0].subtasks.len(), 1); // 1.1.1
        assert_eq!(tasks[0].subtasks[0].subtasks[0].title, "sub of one's sub");

        assert_eq!(tasks[1].title, "second task");
        assert_eq!(tasks[1].subtasks.len(), 2); // 2.1 and 2.2
        assert_eq!(tasks[1].subtasks[0].title, "sub for 2");
        assert_eq!(tasks[1].subtasks[0].subtasks.len(), 1); // 2.1.1
        assert_eq!(tasks[1].subtasks[0].subtasks[0].title, "sub of 2's one");
    }

    #[test]
    fn test_parse_bulk_tasks_syntax_error() {
        let input = "Main\n    Jumped Sub";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let res = parse_bulk_tasks(input, today, Priority::Medium, "📝".to_string());
        assert!(res.is_err());
    }
}
