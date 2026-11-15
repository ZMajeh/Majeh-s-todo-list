use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use crate::components::task_list::{TaskList, TodoTask, TodoSubtask};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use chrono::Datelike;
use gloo_net::http::Request;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let navigate = use_navigate();
    let selected_date = signal(chrono::Local::now().date_naive());
    
    // Load user profile from localStorage
    let user_email = if let Some(window) = web_sys::window() {
        window.local_storage().ok().flatten().and_then(|s| s.get_item("user_email").ok().flatten()).unwrap_or_else(|| "majeh@gmail.com".to_string())
    } else {
        "majeh@gmail.com".to_string()
    };

    let user_name = if let Some(window) = web_sys::window() {
        window.local_storage().ok().flatten().and_then(|s| s.get_item("user_name").ok().flatten()).unwrap_or_else(|| "Majeh".to_string())
    } else {
        "Majeh".to_string()
    };

    let show_settings = signal(false);
    let show_calendar_modal = signal(false);
    let show_notifications_modal = signal(false);
    let priority_filter = signal("All".to_string());
    let tag_filter = signal("All Tags".to_string());
    let due_filter = signal("Due Date".to_string());
    let show_priority_dropdown = signal(false);
    let show_tag_dropdown = signal(false);
    let show_due_dropdown = signal(false);
    let is_syncing = signal(false);
    let sync_progress = signal(0);
    let sync_status = signal("Up to date".to_string());
    let static_subtask_visible = signal(if let Some(window) = web_sys::window() {
        window.local_storage().ok().flatten().and_then(|s| s.get_item("majeh_static_subtask_visible").ok().flatten()).map(|v| v == "true").unwrap_or(false)
    } else {
        false
    });

    let editable_email = signal(user_email.clone());
    let editable_name = signal(user_name.clone());

    let save_profile = move |_| {
        let email = editable_email.0.get();
        let name = editable_name.0.get();
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("user_email", &email);
                let _ = storage.set_item("user_name", &name);
            }
        }
        show_settings.1.set(false);
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
        }
    };

    // Load initial tasks from localStorage or defaults
    let initial_tasks = crate::components::task_list::load_or_default_tasks();

    let tasks = signal(initial_tasks.clone());
    let selected_task = signal(initial_tasks.first().cloned());
    let search_query = signal(String::new());
    let active_tab = signal("active".to_string()); // "active", "completed", "all"

    let prev_day = move |_| {
        let date = selected_date.0.get() - chrono::Duration::days(1);
        selected_date.1.set(date);
    };

    let next_day = move |_| {
        let date = selected_date.0.get() + chrono::Duration::days(1);
        selected_date.1.set(date);
    };

    let logout = {
        let navigate = navigate.clone();
        move |_| {
            navigate("/login", Default::default());
        }
    };

    let set_tab = move |tab: &str| {
        let t = tab.to_string();
        active_tab.1.set(t);
    };

    let toggle_theme = move |dark: bool| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(root) = document.document_element() {
                    if dark {
                        let _ = root.set_attribute("data-theme", "dark");
                    } else {
                        let _ = root.remove_attribute("data-theme");
                    }
                }
            }
        }
    };

    let trigger_sync = move |_| {
        if is_syncing.0.get() { return; }
        is_syncing.1.set(true);
        sync_progress.1.set(10);
        sync_status.1.set("Connecting to Google Drive...".to_string());

        let tasks_signal = tasks.1;
        let tasks_read = tasks.0;

        wasm_bindgen_futures::spawn_local(async move {
            let window = match web_sys::window() {
                Some(w) => w,
                None => {
                    is_syncing.1.set(false);
                    return;
                }
            };

            let storage = window.local_storage().ok().flatten();
            let access_token = storage
                .as_ref()
                .and_then(|s| s.get_item("google_access_token").ok().flatten());

            let token = match access_token {
                Some(t) => t,
                None => {
                    gloo_timers::future::TimeoutFuture::new(500).await;
                    sync_progress.1.set(100);
                    sync_status.1.set("Sync complete! (Local mode - sign in with Google for Drive sync)".to_string());
                    gloo_timers::future::TimeoutFuture::new(1500).await;
                    is_syncing.1.set(false);
                    return;
                }
            };

            sync_progress.1.set(30);
            sync_status.1.set("Searching cloud file on Google Drive...".to_string());

            let search_url = "https://www.googleapis.com/drive/v3/files?q=name='majeh_todo_data.json' and trashed=false";
            let search_res = Request::get(search_url)
                .header("Authorization", &format!("Bearer {}", token))
                .send()
                .await;

            let mut file_id: Option<String> = None;
            let mut cloud_tasks: Vec<TodoTask> = Vec::new();

            if let Ok(resp) = search_res {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(files) = json.get("files").and_then(|v| v.as_array()) {
                        if let Some(file) = files.first() {
                            if let Some(id) = file.get("id").and_then(|v| v.as_str()) {
                                file_id = Some(id.to_string());
                            }
                        }
                    }
                }
            }

            sync_progress.1.set(50);
            sync_status.1.set("Downloading cloud changes...".to_string());

            if let Some(ref fid) = file_id {
                let download_url = &format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", fid);
                let download_res = Request::get(download_url)
                    .header("Authorization", &format!("Bearer {}", token))
                    .send()
                    .await;

                if let Ok(resp) = download_res {
                    if let Ok(tasks_json) = resp.json::<Vec<TodoTask>>().await {
                        cloud_tasks = tasks_json;
                    }
                }
            } else {
                sync_status.1.set("Creating cloud sync file...".to_string());
                let create_url = "https://www.googleapis.com/drive/v3/files";
                let metadata = serde_json::json!({
                    "name": "majeh_todo_data.json",
                    "mimeType": "application/json"
                });
                if let Ok(req) = Request::post(create_url)
                    .header("Authorization", &format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(metadata.to_string()) {
                    if let Ok(resp) = req.send().await {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                                file_id = Some(id.to_string());
                            }
                        }
                    }
                }
            }

            sync_progress.1.set(70);
            sync_status.1.set("Merging local and cloud data...".to_string());

            let local_tasks = tasks_read.get();
            let mut merged_map = std::collections::HashMap::new();
            for t in local_tasks {
                merged_map.insert(t.id.clone(), t);
            }
            // Cloud tasks take precedence so updates from Drive reflect locally
            for t in cloud_tasks {
                merged_map.insert(t.id.clone(), t);
            }
            let mut merged_tasks: Vec<TodoTask> = merged_map.into_values().collect();
            merged_tasks.sort_by(|a, b| b.created_date.cmp(&a.created_date));

            tasks_signal.set(merged_tasks.clone());
            crate::components::task_list::save_to_local_storage(&merged_tasks);

            sync_progress.1.set(90);
            sync_status.1.set("Uploading sync data to Google Drive...".to_string());

            if let Some(ref fid) = file_id {
                let upload_url = &format!("https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media", fid);
                let json_data = serde_json::to_string(&merged_tasks).unwrap_or_default();
                if let Ok(req) = Request::put(upload_url)
                    .header("Authorization", &format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(json_data) {
                    let _ = req.send().await;
                }
            }

            sync_progress.1.set(100);
            sync_status.1.set("Sync complete! Cloud updates reflected locally.".to_string());

            gloo_timers::future::TimeoutFuture::new(1500).await;
            is_syncing.1.set(false);
        });
    };

    let export_tasks = move |_| {
        let list = tasks.0.get();
        if let Ok(json) = serde_json::to_string_pretty(&list) {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let blob = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&json.into()))
                        .expect("Failed to create blob");
                    let url = web_sys::Url::create_object_url_with_blob(&blob)
                        .expect("Failed to create object URL");
                    
                    let a = document.create_element("a").expect("Failed to create element");
                    let a = a.dyn_into::<web_sys::HtmlAnchorElement>().expect("Failed to cast to a");
                    a.set_href(&url);
                    a.set_download("majeh_todo_backup.json");
                    a.click();
                    let _ = web_sys::Url::revoke_object_url(&url);
                }
            }
        }
    };

    let import_tasks = move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let input = document.create_element("input").expect("Failed to create element");
                let input = input.dyn_into::<web_sys::HtmlInputElement>().expect("Failed to cast to input");
                input.set_type("file");
                input.set_accept(".json");
                
                let t_write = tasks.1;
                let on_change = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                    let input = ev.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                    if let Some(files) = input.files() {
                        if let Some(file) = files.get(0) {
                            let reader = web_sys::FileReader::new().expect("Failed to create reader");
                            let reader_c = reader.clone();
                            let on_load = Closure::wrap(Box::new(move |_| {
                                if let Ok(res) = reader_c.result() {
                                    if let Some(s) = res.as_string() {
                                        if let Ok(imported) = serde_json::from_str::<Vec<TodoTask>>(&s) {
                                            t_write.set(imported.clone());
                                            if let Some(window) = web_sys::window() {
                                                if let Ok(Some(storage)) = window.local_storage() {
                                                    if let Ok(json) = serde_json::to_string(&imported) {
                                                        let _ = storage.set_item("majeh_tasks", &json);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }) as Box<dyn FnMut(web_sys::Event)>);
                            reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                            on_load.forget();
                            let _ = reader.read_as_text(&file);
                        }
                    }
                }) as Box<dyn FnMut(web_sys::Event)>);
                
                input.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref()).expect("Failed to add listener");
                on_change.forget();
                input.click();
            }
        }
    };

    let _user_email_c1 = user_email.clone();
    let user_name_c1 = user_name.clone();
    let sidebar_hidden = signal(false);
    let sidebar_mobile_open = signal(false);

    view! {
        <div class="app-container">
            // Left Sidebar
            {let user_email_sidebar = user_email.clone();
             let user_name_sidebar = user_name.clone();
             let logout_sidebar = logout.clone();
             move || {
                let logout = logout_sidebar.clone();
                let user_email_c1 = user_email_sidebar.clone();
                let user_name_c1 = user_name_sidebar.clone();
                let cls = if sidebar_mobile_open.0.get() { "sidebar mobile-open" } else { "sidebar" };
                let display_style = if sidebar_hidden.0.get() { "display: none;" } else { "" };
                view! {
                    <aside class=cls style=display_style>
                        <div>
                            <div class="sidebar-header" style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px;">
                                <div style="display: flex; align-items: center; gap: 6px;">
                                    <span style="width: 12px; height: 12px; border-radius: 50%; background: #ef4444; display: inline-block;"></span>
                                    <span style="width: 12px; height: 12px; border-radius: 50%; background: #f59e0b; display: inline-block;"></span>
                                    <span style="width: 12px; height: 12px; border-radius: 50%; background: #10b981; display: inline-block;"></span>
                                </div>
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <span
                                        style="color: var(--text-muted); cursor: pointer; font-size: 14px;"
                                        title="Close Menu"
                                        on:click=move |_| sidebar_mobile_open.1.set(false)
                                    >
                                        "✕"
                                    </span>
                                    <span
                                        style="color: var(--text-muted); cursor: pointer; font-size: 14px;"
                                        title="Collapse Sidebar"
                                        on:click=move |_| sidebar_hidden.1.set(true)
                                    >
                                        "«"
                                    </span>
                                </div>
                            </div>
                            <div class="logo-area" style="margin-bottom: 20px;">
                                <div class="logo-icon">"✓"</div>
                                <span class="logo-text">"Majeh's TODO"</span>
                            </div>
                            <nav class="sidebar-nav">
                                <button
                                    class=move || if active_tab.0.get() == "all" { "nav-item active" } else { "nav-item" }
                                    on:click=move |_| { set_tab("all"); sidebar_mobile_open.1.set(false); }
                                >
                                    <span>"✓"</span> "Tasks"
                                </button>
                                <button
                                    class=move || if active_tab.0.get() == "completed" { "nav-item active" } else { "nav-item" }
                                    on:click=move |_| { set_tab("completed"); sidebar_mobile_open.1.set(false); }
                                >
                                    <span>"☑"</span> "Completed"
                                </button>
                                <button class="nav-item" on:click=move |_| { show_calendar_modal.1.set(true); sidebar_mobile_open.1.set(false); }><span>"📅"</span> "Calendar"</button>
                                <button class="nav-item" on:click=move |_| { trigger_sync(()); sidebar_mobile_open.1.set(false); }><span>"🔄"</span> "Sync"</button>
                                <button class="nav-item" on:click=move |_| { import_tasks(()); sidebar_mobile_open.1.set(false); }><span>"📥"</span> "Import"</button>
                                <button class="nav-item" on:click=move |_| { export_tasks(()); sidebar_mobile_open.1.set(false); }><span>"📤"</span> "Export"</button>
                                <button class="nav-item" on:click=move |_| { show_settings.1.set(true); sidebar_mobile_open.1.set(false); }><span>"⚙️"</span> "Settings"</button>
                            </nav>
                        </div>
                        <div class="sidebar-footer">
                            <div style="display: flex; gap: 8px;">
                                <button class="nav-item" style="flex: 1; justify-content: center; padding: 8px; font-size: 12px; text-align: center;" on:click=move |_| toggle_theme(false)>"☀️ Light"</button>
                                <button class="nav-item" style="flex: 1; justify-content: center; padding: 8px; font-size: 12px; text-align: center;" on:click=move |_| toggle_theme(true)>"🌙 Dark"</button>
                            </div>
                            <div class="drive-status">
                                <div style="font-weight: 600; margin-bottom: 4px;">"Google Drive"</div>
                                <div style="color: var(--text-muted); font-size: 12px;">{format!("Connected ({})", user_email_c1)}</div>
                                <div style="background: var(--border-color); height: 4px; border-radius: 2px; margin-top: 8px; overflow: hidden;">
                                    <div style="background: var(--primary); width: 25%; height: 100%;"></div>
                                </div>
                            </div>
                            <div class="user-profile-card">
                                <div class="user-info">
                                    <div class="user-avatar">"M"</div>
                                    <div>
                                        <div style="font-weight: 600; font-size: 14px;">{user_name_c1.clone()}</div>
                                        <div style="color: var(--text-muted); font-size: 12px;">{user_email_c1.clone()}</div>
                                    </div>
                                </div>
                                <button
                                    class="nav-btn"
                                    title="Logout"
                                    on:click=logout
                                >
                                    "⏻"
                                </button>
                            </div>
                        </div>
                    </aside>
                }.into_any()
            }}

            // Main Dashboard Content
            <main class="main-content">
                <header class="dashboard-header">
                    <div class="greeting" style="display: flex; align-items: center; gap: 12px;">
                        <button
                            class="nav-btn mobile-hamburger"
                            title="Open Menu"
                            style="width: 36px; height: 36px; border-radius: 8px; display: none; align-items: center; justify-content: center; background: var(--bg-card); border: 1px solid var(--border-color); cursor: pointer; font-size: 18px;"
                            on:click=move |_| sidebar_mobile_open.1.update(|v| *v = !*v)
                        >
                            "☰"
                        </button>
                        {move || {
                            if sidebar_hidden.0.get() {
                                view! {
                                    <button
                                        class="nav-btn"
                                        title="Show Sidebar"
                                        style="width: 36px; height: 36px; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: var(--bg-card); border: 1px solid var(--border-color); cursor: pointer; font-size: 16px;"
                                        on:click=move |_| sidebar_hidden.1.set(false)
                                    >
                                        "»"
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span /> }.into_any()
                            }
                        }}
                        <div>
                            {let u_name = user_name_c1.clone();
                            view! {
                                <>
                                    <h1>{format!("Good Morning, {}!", u_name)}</h1>
                                    <p>"Let's make today productive."</p>
                                </>
                            }}
                        </div>
                    </div>
                    <div class="header-actions" style="display: flex; align-items: center; gap: 16px;">
                        <div class="search-bar" style="display: flex; align-items: center; gap: 8px;">
                            <span>"🔍"</span>
                            <input
                                type="text"
                                placeholder="Search tasks, tags, or notes..."
                                on:input=move |ev| search_query.1.set(event_target_value(&ev))
                                prop:value=search_query.0
                            />
                            <span style="font-size: 11px; background: var(--bg-hover); border: 1px solid var(--border-color); padding: 2px 6px; border-radius: 4px; color: var(--text-muted); font-weight: 500;">"Ctrl + K"</span>
                        </div>
                        <button class="nav-btn" title="Notifications" style="width: 36px; height: 36px; border-radius: 50%; display: flex; align-items: center; justify-content: center; background: var(--bg-card); border: 1px solid var(--border-color);" on:click=move |_| show_notifications_modal.1.set(true)>"🔔"</button>
                    </div>
                </header>

                // Sync Progress Banner (when syncing)
                {move || {
                    if is_syncing.0.get() {
                        let pct = sync_progress.0.get();
                        let status = sync_status.0.get();
                        view! {
                            <div style="background: var(--primary-light); border: 1px solid var(--primary); padding: 14px 18px; border-radius: var(--radius); margin-bottom: 20px; font-size: 14px; box-shadow: var(--shadow);">
                                <div style="display: flex; justify-content: space-between; font-weight: 600; color: var(--primary); margin-bottom: 8px;">
                                    <span>{status}</span>
                                    <span>{format!("{}%", pct)}</span>
                                </div>
                                <div style="background: var(--border-color); height: 8px; border-radius: 4px; overflow: hidden;">
                                    <div style=format!("background: var(--primary); width: {}%; height: 100%; transition: width 0.3s ease;", pct)></div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}

                <div class="quote-box">
                    "\"Small steps every day lead to big results.\" — Unknown"
                </div>

                <div class="date-nav-bar" style="display: flex; align-items: center; gap: 12px;">
                    <div style="display: flex; align-items: center; gap: 4px;">
                        <button class="nav-btn" on:click=prev_day>"<"</button>
                        <div class="date-display" style="display: flex; flex-direction: column; align-items: center; line-height: 1.2; padding: 0 12px;">
                            <span style="font-weight: 700; font-size: 15px;">{move || selected_date.0.get().format("%B %d, %Y").to_string()}</span>
                            <span style="font-size: 11px; color: var(--text-muted);">{move || format!("{}, Week {}", selected_date.0.get().format("%A"), selected_date.0.get().iso_week().week())}</span>
                        </div>
                        <button class="nav-btn" on:click=next_day>">"</button>
                    </div>
                    <button class="nav-btn" style="width: auto; padding: 6px 14px; border-radius: 6px; font-size: 13px;" on:click=move |_| selected_date.1.set(chrono::Local::now().date_naive())>"Today"</button>
                    <button class="nav-btn" style="width: 36px; height: 36px; border-radius: 6px; display: flex; align-items: center; justify-content: center;" title="Calendar View" on:click=move |_| show_calendar_modal.1.set(true)>"📅"</button>
                </div>

                <div class="task-filters" style="display: flex; justify-content: space-between; align-items: center; position: relative;">
                    <div style="display: flex; gap: 12px;">
                        <button
                            class=move || if active_tab.0.get() == "active" { "filter-tab active" } else { "filter-tab" }
                            on:click=move |_| set_tab("active")
                        >
                            {move || format!("Active ({})", tasks.0.get().iter().filter(|t| !t.completed).count())}
                        </button>
                        <button
                            class=move || if active_tab.0.get() == "completed" { "filter-tab active" } else { "filter-tab" }
                            on:click=move |_| set_tab("completed")
                        >
                            {move || format!("Completed ({})", tasks.0.get().iter().filter(|t| t.completed).count())}
                        </button>
                        <button
                            class=move || if active_tab.0.get() == "all" { "filter-tab active" } else { "filter-tab" }
                            on:click=move |_| set_tab("all")
                        >
                            {move || format!("All ({})", tasks.0.get().len())}
                        </button>
                    </div>
                    <div style="display: flex; gap: 8px;">
                        <div style="position: relative;">
                            <button class="nav-btn" style="width: auto; padding: 0 12px; border-radius: 6px; font-size: 13px;" on:click=move |_| show_priority_dropdown.1.update(|v| *v = !*v)>
                                {move || format!("Priority: {} ▾", priority_filter.0.get())}
                            </button>
                            {move || {
                                if show_priority_dropdown.0.get() {
                                    view! {
                                        <div style="position: absolute; right: 0; top: 32px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 120px; font-size: 13px;">
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| { priority_filter.1.set("All".to_string()); show_priority_dropdown.1.set(false); }>"All"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { priority_filter.1.set("High".to_string()); show_priority_dropdown.1.set(false); }>"High"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { priority_filter.1.set("Medium".to_string()); show_priority_dropdown.1.set(false); }>"Medium"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { priority_filter.1.set("Low".to_string()); show_priority_dropdown.1.set(false); }>"Low"</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}
                        </div>

                        <div style="position: relative;">
                            <button class="nav-btn" style="width: auto; padding: 0 12px; border-radius: 6px; font-size: 13px;" on:click=move |_| show_tag_dropdown.1.update(|v| *v = !*v)>
                                {move || format!("Tag: {} ▾", tag_filter.0.get())}
                            </button>
                            {move || {
                                if show_tag_dropdown.0.get() {
                                    view! {
                                        <div style="position: absolute; right: 0; top: 32px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 140px; font-size: 13px;">
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| { tag_filter.1.set("All Tags".to_string()); show_tag_dropdown.1.set(false); }>"All Tags"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { tag_filter.1.set("Website".to_string()); show_tag_dropdown.1.set(false); }>"Website"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { tag_filter.1.set("Personal".to_string()); show_tag_dropdown.1.set(false); }>"Personal"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main); border-top: 1px solid var(--border-color);" on:click=move |_| { tag_filter.1.set("Development".to_string()); show_tag_dropdown.1.set(false); }>"Development"</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}
                        </div>

                        <div style="position: relative;">
                            <button class="nav-btn" style="width: auto; padding: 0 12px; border-radius: 6px; font-size: 13px;" on:click=move |_| show_due_dropdown.1.update(|v| *v = !*v)>
                                {move || format!("Due: {} ▾", due_filter.0.get())}
                            </button>
                            {move || {
                                if show_due_dropdown.0.get() {
                                    view! {
                                        <div style="position: absolute; right: 0; top: 32px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; box-shadow: var(--shadow); z-index: 100; display: flex; flex-direction: column; width: 130px; font-size: 13px;">
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| { due_filter.1.set("Due Date".to_string()); show_due_dropdown.1.set(false); }>"All Dates"</button>
                                            <button style="background: transparent; border: 1px solid var(--border-color); padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| { due_filter.1.set("Today".to_string()); show_due_dropdown.1.set(false); }>"Today"</button>
                                            <button style="background: transparent; border: none; padding: 8px 12px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| { due_filter.1.set("Upcoming".to_string()); show_due_dropdown.1.set(false); }>"Upcoming"</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </div>

                <TaskList
                    tasks=tasks.0
                    set_tasks=tasks.1
                    selected_task=selected_task.0
                    set_selected_task=selected_task.1
                    selected_date=selected_date.0
                    search_query=search_query.0
                    active_tab=active_tab.0
                    priority_filter=priority_filter.0
                    tag_filter=tag_filter.0
                    due_filter=due_filter.0
                    static_subtask_visible=static_subtask_visible.0
                />
            </main>

            // Calendar Modal
            {move || {
                if show_calendar_modal.0.get() {
                    view! {
                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 420px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <h3 style="font-size: 18px; font-weight: 700;">"📅 Calendar & Schedule"</h3>
                                    <button class="close-btn" on:click=move |_| show_calendar_modal.1.set(false)>"×</"</button>
                                </div>
                                <div style="text-align: center; font-size: 15px; font-weight: 600; color: var(--primary);">
                                    {selected_date.0.get().format("%B %Y").to_string()}
                                </div>
                                <p style="font-size: 13px; color: var(--text-muted); text-align: center;">"Select a date or view tasks scheduled for this month."</p>
                                <div style="display: flex; justify-content: flex-end;">
                                    <button class="google-login-btn" on:click=move |_| show_calendar_modal.1.set(false)>"Close Calendar"</button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Notifications Modal
            {move || {
                if show_notifications_modal.0.get() {
                    view! {
                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 420px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <h3 style="font-size: 18px; font-weight: 700;">"🔔 Notifications"</h3>
                                    <button class="close-btn" on:click=move |_| show_notifications_modal.1.set(false)>"×</"</button>
                                </div>
                                <div style="display: flex; flex-direction: column; gap: 10px; font-size: 13px;">
                                    <div style="display: flex; gap: 10px; align-items: center; padding: 10px; background: var(--bg-hover); border-radius: 8px;">
                                        <span style="color: var(--primary); font-size: 16px;">"✓"</span>
                                        <div>
                                            <div style="font-weight: 600;">"Google Drive Sync successful"</div>
                                            <div style="color: var(--text-muted); font-size: 11px;">"Today, 10:00 AM"</div>
                                        </div>
                                    </div>
                                    <div style="display: flex; gap: 10px; align-items: center; padding: 10px; background: var(--bg-hover); border-radius: 8px;">
                                        <span style="color: #f59e0b; font-size: 16px;">"⚑"</span>
                                        <div>
                                            <div style="font-weight: 600;">"Build Website due today"</div>
                                            <div style="color: var(--text-muted); font-size: 11px;">"Today, 09:00 AM"</div>
                                        </div>
                                    </div>
                                </div>
                                <button class="google-login-btn" on:click=move |_| show_notifications_modal.1.set(false)>"Close"</button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Settings Modal
            {move || {
                if show_settings.0.get() {
                    let _t_sync = trigger_sync.clone();
                    view! {
                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 450px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <h2 style="font-size: 18px; font-weight: 700;">"Settings"</h2>
                                    <button class="close-btn" on:click=move |_| show_settings.1.set(false)>"×"</button>
                                </div>
                                <div style="display: flex; flex-direction: column; gap: 16px; font-size: 14px;">
                                    <div style="display: flex; flex-direction: column; gap: 6px;">
                                        <label style="color: var(--text-muted); font-size: 13px;">"Account Email"</label>
                                        <input
                                            type="text"
                                            class="task-input"
                                            style="padding: 10px; font-size: 14px;"
                                            prop:value=editable_email.0
                                            on:input=move |ev| editable_email.1.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div style="display: flex; flex-direction: column; gap: 6px;">
                                        <label style="color: var(--text-muted); font-size: 13px;">"User Name"</label>
                                        <input
                                            type="text"
                                            class="task-input"
                                            style="padding: 10px; font-size: 14px;"
                                            prop:value=editable_name.0
                                            on:input=move |ev| editable_name.1.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <button class="add-task-btn" style="padding: 10px; font-size: 14px;" on:click=save_profile>"Save Profile"</button>

                                    <div>
                                        <span style="color: var(--text-muted); font-size: 13px; display: block; margin-bottom: 4px;">"Google Drive Sync"</span>
                                        <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-hover); padding: 10px; border-radius: 8px;">
                                            <span>"Connected & Active"</span>
                                            <button class="add-task-btn" style="padding: 6px 12px; font-size: 12px;" on:click=move |_| trigger_sync(())>"Sync Now"</button>
                                        </div>
                                    </div>
                                    <div>
                                        <span style="color: var(--text-muted); font-size: 13px; display: block; margin-bottom: 8px;">"Subtasks Display"</span>
                                        <div style="display: flex; align-items: center; gap: 8px; background: var(--bg-hover); padding: 10px; border-radius: 8px;">
                                            <input
                                                type="checkbox"
                                                id="static-subtask-checkbox"
                                                prop:checked=static_subtask_visible.0
                                                on:change=move |ev| {
                                                    let checked = ev.target()
                                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                                        .map(|input| input.checked())
                                                        .unwrap_or(false);
                                                    static_subtask_visible.1.set(checked);
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            let _ = storage.set_item("majeh_static_subtask_visible", &checked.to_string());
                                                        }
                                                    }
                                                }
                                            />
                                            <label for="static-subtask-checkbox" style="font-size: 13px; font-weight: 500; cursor: pointer;">"Always show subtask add inputs (disable hover)"</label>
                                        </div>
                                    </div>
                                    <div>
                                        <span style="color: var(--text-muted); font-size: 13px; display: block; margin-bottom: 8px;">"Theme Mode"</span>
                                        <div style="display: flex; gap: 10px;">
                                            <button class="nav-item" style="justify-content: center; background: var(--bg-hover); text-align: center; border: 1px solid var(--border-color);" on:click=move |_| toggle_theme(false)>"☀️ Light Mode"</button>
                                            <button class="nav-item" style="justify-content: center; background: var(--bg-hover); text-align: center; border: 1px solid var(--border-color);" on:click=move |_| toggle_theme(true)>"🌙 Dark Mode"</button>
                                        </div>
                                    </div>
                                </div>
                                <button class="google-login-btn" on:click=move |_| show_settings.1.set(false)>"Close Settings"</button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Right Details Panel (Interactive)
            {move || {
                let current_sel = selected_task.0.get();
                if let Some(task) = current_sel {
                    let _task_id_notes = task.id.clone();
                    let task_id_sub = task.id.clone();
                    let task_id_toggle = task.id.clone();
                    let _task_id_icon = task.id.clone();
                    let _task_id_pri = task.id.clone();
                    let _task_id_due = task.id.clone();
                    let task_id_tags = task.id.clone();
                    let _task_id_del = task.id.clone();
                    let task_id_dup = task.id.clone();
                    let task_title = task.title.clone();
                    let task_desc = task.description.clone();
                    let task_icon = task.icon.clone();
                    let task_priority_enum = task.priority.clone();
                    let task_priority_raw = format!("{:?}", task_priority_enum);
                    let task_priority = match task_priority_enum {
                        crate::components::task_list::Priority::High => "⚑ High".to_string(),
                        crate::components::task_list::Priority::Medium => "⚑ Medium".to_string(),
                        crate::components::task_list::Priority::Low => "⚑ Low".to_string(),
                    };
                    let task_priority_class = format!("priority-badge priority-{}", task_priority_raw.to_lowercase());
                    let task_notes = task.notes.clone();
                    let task_tags = task.tags.clone();
                    let subtasks = task.subtasks.clone();
                    let completed_subtasks = subtasks.iter().filter(|s| s.completed).count();
                    let total_subtasks = subtasks.len();
                    let progress_pct = if total_subtasks > 0 {
                        (completed_subtasks * 100) / total_subtasks
                    } else {
                        0
                    };
                    let due_date_str = task.due_date.map(|d| format!("{}", d)).unwrap_or_else(|| "No due date".to_string());
                    let has_due_date = task.due_date.is_some();
                    let notes_len = task_notes.len();

                    let show_options_menu = signal(false);
                    let show_activity_modal = signal(false);
                    let new_sub_title = signal(String::new());
                    let new_sub_pri = signal(crate::components::task_list::Priority::Medium);

                    let cycle_icon = move |_| {
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            let icons = vec!["💻", "📄", "✈️", "⚙️", "📖", "📝", "🚀", "💡"];
                            let current = t.icon.clone();
                            let idx = icons.iter().position(|&x| x == current).unwrap_or(0);
                            let next_icon = icons[(idx + 1) % icons.len()].to_string();
                            tasks.1.update(|ts| {
                                if let Some(task) = ts.iter_mut().find(|x| x.id == id) {
                                    task.icon = next_icon;
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                        }
                    };

                    let cycle_priority = move |_| {
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            let next_pri = match t.priority {
                                crate::components::task_list::Priority::High => crate::components::task_list::Priority::Medium,
                                crate::components::task_list::Priority::Medium => crate::components::task_list::Priority::Low,
                                crate::components::task_list::Priority::Low => crate::components::task_list::Priority::High,
                            };
                            tasks.1.update(|ts| {
                                if let Some(task) = ts.iter_mut().find(|x| x.id == id) {
                                    task.priority = next_pri;
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                        }
                    };

                    let clear_due_date = move |_| {
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            tasks.1.update(|ts| {
                                if let Some(task) = ts.iter_mut().find(|x| x.id == id) {
                                    task.due_date = None;
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                        }
                    };

                    let remove_tag = move |tag_to_rem: String| {
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            tasks.1.update(|ts| {
                                if let Some(task) = ts.iter_mut().find(|x| x.id == id) {
                                    task.tags.retain(|tag| tag != &tag_to_rem);
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                        }
                    };

                    let show_add_tag_modal = signal(false);
                    let new_tag_val = signal(String::new());

                    let add_tag = move |_| {
                        show_add_tag_modal.1.set(true);
                    };

                    let delete_task_detail = move |_| {
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            tasks.1.update(|ts| {
                                ts.retain(|x| x.id != id);
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            selected_task.1.set(None);
                        }
                    };

                    let update_notes = move |ev: web_sys::Event| {
                        let val = event_target_value(&ev);
                        if let Some(t) = selected_task.0.get() {
                            let id = t.id;
                            tasks.1.update(|ts| {
                                if let Some(task) = ts.iter_mut().find(|x| x.id == id) {
                                    task.notes = val.clone();
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                        }
                    };

                    let add_subtask = move |_| {
                        let title = new_sub_title.0.get();
                        let pri = new_sub_pri.0.get();
                        let id = task_id_sub.clone();
                        let parent_due = task.due_date;
                        if !title.is_empty() {
                            tasks.1.update(|ts| {
                                if let Some(t) = ts.iter_mut().find(|x| x.id == id) {
                                    t.subtasks.push(TodoSubtask {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        title,
                                        completed: false,
                                        priority: pri,
                                        due_date: parent_due,
                                        subtasks: vec![],
                                    });
                                }
                                if let Some(window) = web_sys::window() {
                                    if let Ok(Some(storage)) = window.local_storage() {
                                        if let Ok(json) = serde_json::to_string(ts) {
                                            let _ = storage.set_item("majeh_tasks", &json);
                                        }
                                    }
                                }
                            });
                            let current_tasks = tasks.0.get();
                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                selected_task.1.set(Some(updated.clone()));
                            }
                            new_sub_title.1.set(String::new());
                        }
                    };

                    view! {
                        <aside class="detail-panel">
                            <div class="detail-header">
                                <div style="display: flex; align-items: center; gap: 8px; cursor: pointer;" title="Click to change icon" on:click=cycle_icon>
                                    <span style="font-size: 18px;">{task_icon}</span>
                                    <h3 style="font-size: 16px; font-weight: 600;">"Task Details"</h3>
                                </div>
                                <div style="display: flex; align-items: center; gap: 8px; position: relative;">
                                    <button class="close-btn" style="font-size: 16px;" on:click=move |_| show_options_menu.1.update(|v| *v = !*v)>"⋯"</button>
                                    {let task_id_dup = task_id_dup.clone();
                                    move || {
                                        let task_id_dup = task_id_dup.clone();
                                        if show_options_menu.0.get() {
                                            view! {
                                                <div style="position: absolute; right: 0; top: 30px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; box-shadow: var(--shadow); z-index: 200; display: flex; flex-direction: column; width: 140px; font-size: 13px;">
                                                    <button style="background: transparent; border: none; padding: 10px 14px; text-align: left; cursor: pointer; color: var(--text-main);" on:click=move |_| {
                                                        let id = task_id_dup.clone();
                                                        tasks.1.update(|ts| {
                                                            if let Some(t) = ts.iter().find(|x| x.id == id) {
                                                                let mut cloned = t.clone();
                                                                cloned.id = uuid::Uuid::new_v4().to_string();
                                                                cloned.title = format!("{} (Copy)", cloned.title);
                                                                ts.push(cloned);
                                                            }
                                                            if let Some(window) = web_sys::window() {
                                                                if let Ok(Some(storage)) = window.local_storage() {
                                                                    if let Ok(json) = serde_json::to_string(ts) {
                                                                        let _ = storage.set_item("majeh_tasks", &json);
                                                                    }
                                                                }
                                                            }
                                                        });
                                                        show_options_menu.1.set(false);
                                                    }>"Duplicate"</button>
                                                    <button style="background: transparent; border: none; padding: 10px 14px; text-align: left; cursor: pointer; color: #dc2626; border-top: 1px solid var(--border-color);" on:click=delete_task_detail>"Delete Task"</button>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span /> }.into_any()
                                        }
                                    }}
                                    <button
                                        class="close-btn"
                                        on:click=move |_| selected_task.1.set(None)
                                    >
                                        "×"
                                    </button>
                                </div>
                            </div>

                            <div style="display: flex; flex-direction: column; gap: 20px; margin-top: 10px;">
                                <div>
                                    <div style="font-size: 18px; font-weight: 700; color: var(--text-main);">{task_title}</div>
                                    <div style="font-size: 13px; color: var(--text-muted); margin-top: 4px;">{task_desc}</div>
                                </div>

                                // Progress Section
                                <div>
                                    <div style="display: flex; justify-content: space-between; font-size: 12px; color: var(--text-muted); margin-bottom: 6px;">
                                        <span>{format!("{} of {} subtasks completed", completed_subtasks, total_subtasks)}</span>
                                        <span>{format!("{}%", progress_pct)}</span>
                                    </div>
                                    <div style="background: var(--border-color); height: 6px; border-radius: 3px; overflow: hidden;">
                                        <div style=format!("background: var(--primary); width: {}%; height: 100%;", progress_pct)></div>
                                    </div>
                                </div>

                                // Attributes
                                <div style="display: flex; flex-direction: column; gap: 12px; font-size: 14px;">
                                    <div style="display: flex; justify-content: space-between; align-items: center;">
                                        <span style="color: var(--text-muted); font-size: 13px;">"Due Date"</span>
                                        <div style="background: var(--bg-hover); padding: 4px 10px; border-radius: 6px; font-size: 13px; display: flex; align-items: center; gap: 6px;">
                                            <span>{due_date_str}</span>
                                            {if has_due_date {
                                                view! { <span style="cursor: pointer; color: var(--text-muted); font-weight: bold;" on:click=clear_due_date title="Clear due date">"×"</span> }.into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }}
                                        </div>
                                    </div>

                                    <div style="display: flex; justify-content: space-between; align-items: center;">
                                        <span style="color: var(--text-muted); font-size: 13px;">"Priority"</span>
                                        <span class=task_priority_class style="cursor: pointer;" title="Click to change priority" on:click=cycle_priority>{task_priority}</span>
                                    </div>

                                    <div>
                                        <span style="color: var(--text-muted); font-size: 13px; display: block; margin-bottom: 6px;">"Tags"</span>
                                        <div style="display: flex; flex-wrap: wrap; gap: 6px;">
                                            {task_tags.into_iter().map(|tag| {
                                                let tag_c = tag.clone();
                                                view! {
                                                    <span style="background: var(--primary-light); color: var(--primary); padding: 4px 10px; border-radius: 6px; font-size: 12px; display: flex; align-items: center; gap: 6px;">
                                                        {tag}
                                                        <span style="cursor: pointer; font-weight: bold;" on:click=move |_| remove_tag(tag_c.clone()) title="Remove tag">"×"</span>
                                                    </span>
                                                }
                                            }).collect_view()}
                                            <button style="background: transparent; border: 1px dashed var(--border-color); color: var(--primary); padding: 4px 10px; border-radius: 6px; font-size: 12px; cursor: pointer;" on:click=add_tag>"+ Add tag"</button>
                                        </div>
                                    </div>
                                </div>

                                // Notes Section
                                <div>
                                    <label style="font-size: 13px; color: var(--text-muted); display: block; margin-bottom: 6px;">"Notes"</label>
                                    <textarea
                                        class="task-input"
                                        style="width: 100%; height: 90px; resize: none; font-size: 13px; padding: 10px;"
                                        on:input=update_notes
                                        prop:value=task_notes
                                    ></textarea>
                                    <div style="text-align: right; font-size: 11px; color: var(--text-muted); margin-top: 4px;">
                                        {format!("{}/500", notes_len)}
                                    </div>
                                </div>

                                // Subtasks Section
                                <div>
                                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                                        <span style="font-size: 14px; font-weight: 600;">{format!("Subtasks ({})", total_subtasks)}</span>
                                    </div>
                                    <div style="display: flex; gap: 8px; margin-bottom: 8px;">
                                        <input
                                            type="text"
                                            placeholder="Add subtask..."
                                            on:input=move |ev| new_sub_title.1.set(event_target_value(&ev))
                                            prop:value=new_sub_title.0
                                            class="task-input"
                                            style="font-size: 13px; padding: 8px; flex: 1;"
                                        />
                                        <select
                                            style="background: var(--bg-card); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 6px; padding: 4px; font-size: 12px;"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                let p = match val.as_str() {
                                                    "High" => crate::components::task_list::Priority::High,
                                                    "Low" => crate::components::task_list::Priority::Low,
                                                    _ => crate::components::task_list::Priority::Medium,
                                                };
                                                new_sub_pri.1.set(p);
                                            }
                                        >
                                            <option value="Medium">"Med"</option>
                                            <option value="High">"High"</option>
                                            <option value="Low">"Low"</option>
                                        </select>
                                        <button
                                            on:click=add_subtask
                                            style="background: var(--primary); color: white; border: none; padding: 0 14px; border-radius: 8px; cursor: pointer; font-size: 13px;"
                                        >
                                            "Add"
                                        </button>
                                    </div>
                                    <div style="display: flex; flex-direction: column; gap: 6px;">
                                        {
                                            let task_id_for_toggle = task_id_toggle.clone();
                                            let toggle_subtask_cb = move |tid: String, sid: String| {
                                                tasks.1.update(|ts| {
                                                    if let Some(t) = ts.iter_mut().find(|x| x.id == tid) {
                                                        crate::components::task_list::toggle_subtask_recursive(&mut t.subtasks, &sid);
                                                    }
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            if let Ok(json) = serde_json::to_string(ts) {
                                                                let _ = storage.set_item("majeh_tasks", &json);
                                                            }
                                                        }
                                                    }
                                                });
                                                let current_tasks = tasks.0.get();
                                                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id_for_toggle) {
                                                    selected_task.1.set(Some(updated.clone()));
                                                }
                                            };

                                            let task_id_for_add = task_id_toggle.clone();
                                            let add_sub_recursive_cb = move |tid: String, target_id: String, title: String, pri: crate::components::task_list::Priority, parent_due: Option<chrono::NaiveDate>| {
                                                tasks.1.update(|ts| {
                                                    let new_sub = crate::components::task_list::TodoSubtask {
                                                        id: uuid::Uuid::new_v4().to_string(),
                                                        title,
                                                        completed: false,
                                                        priority: pri,
                                                        due_date: parent_due,
                                                        subtasks: vec![],
                                                    };
                                                    crate::components::task_list::add_subtask_to_task(ts, &tid, Some(&target_id), new_sub);
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            if let Ok(json) = serde_json::to_string(ts) {
                                                                let _ = storage.set_item("majeh_tasks", &json);
                                                            }
                                                        }
                                                    }
                                                });
                                                let current_tasks = tasks.0.get();
                                                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id_for_add) {
                                                    selected_task.1.set(Some(updated.clone()));
                                                }
                                            };

                                            let task_id_for_del = task_id_toggle.clone();
                                            let delete_sub_cb = move |tid: String, sid: String| {
                                                tasks.1.update(|ts| {
                                                    crate::components::task_list::delete_subtask_from_task(ts, &tid, &sid);
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            if let Ok(json) = serde_json::to_string(ts) {
                                                                let _ = storage.set_item("majeh_tasks", &json);
                                                            }
                                                        }
                                                    }
                                                });
                                                let current_tasks = tasks.0.get();
                                                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id_for_del) {
                                                    selected_task.1.set(Some(updated.clone()));
                                                }
                                            };

                                            let task_id_for_pri = task_id_toggle.clone();
                                            let update_priority_cb = move |tid: String, sid: String, new_pri: crate::components::task_list::Priority| {
                                                tasks.1.update(|ts| {
                                                    crate::components::task_list::update_subtask_priority_in_task(ts, &tid, &sid, new_pri);
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            if let Ok(json) = serde_json::to_string(ts) {
                                                                let _ = storage.set_item("majeh_tasks", &json);
                                                            }
                                                        }
                                                    }
                                                });
                                                let current_tasks = tasks.0.get();
                                                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id_for_pri) {
                                                    selected_task.1.set(Some(updated.clone()));
                                                }
                                            };

                                            let task_id_for_due = task_id_toggle.clone();
                                            let update_due_cb = move |tid: String, sid: String, new_due: Option<chrono::NaiveDate>| {
                                                tasks.1.update(|ts| {
                                                    crate::components::task_list::update_subtask_due_date_in_task(ts, &tid, &sid, new_due);
                                                    if let Some(window) = web_sys::window() {
                                                        if let Ok(Some(storage)) = window.local_storage() {
                                                            if let Ok(json) = serde_json::to_string(ts) {
                                                                let _ = storage.set_item("majeh_tasks", &json);
                                                            }
                                                        }
                                                    }
                                                });
                                                let current_tasks = tasks.0.get();
                                                if let Some(updated) = current_tasks.iter().find(|x| x.id == task_id_for_due) {
                                                    selected_task.1.set(Some(updated.clone()));
                                                }
                                            };

                                            crate::components::task_list::render_subtasks_recursive_view(subtasks.clone(), task_id_toggle, task.due_date, toggle_subtask_cb, add_sub_recursive_cb, delete_sub_cb, update_priority_cb, update_due_cb, static_subtask_visible.0, 1, true)
                                        }
                                    </div>
                                </div>

                                // Activity Section
                                <div style="border-top: 1px solid var(--border-color); padding-top: 16px;">
                                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
                                        <span style="font-size: 14px; font-weight: 600;">"Activity"</span>
                                        <span style="font-size: 12px; color: var(--primary); cursor: pointer; font-weight: 500;" on:click=move |_| show_activity_modal.1.set(true)>"Show all"</span>
                                    </div>
                                    <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
                                        <div style="display: flex; gap: 10px; align-items: flex-start;">
                                            <div style="width: 20px; height: 20px; border-radius: 50%; background: #dcfce7; color: #16a34a; display: flex; align-items: center; justify-content: center; font-size: 10px;">"✓"</div>
                                            <div>
                                                <div>"Task created / updated locally"</div>
                                                <div style="color: var(--text-muted); font-size: 11px;">"Today"</div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            // Activity Modal
                            {move || {
                                if show_activity_modal.0.get() {
                                    view! {
                                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 450px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <h3 style="font-size: 16px; font-weight: 700;">"Activity History"</h3>
                                                    <button class="close-btn" on:click=move |_| show_activity_modal.1.set(false)>"×</"</button>
                                                </div>
                                                <div style="display: flex; flex-direction: column; gap: 12px; font-size: 13px;">
                                                    <div style="display: flex; gap: 10px; align-items: center; padding: 8px; background: var(--bg-hover); border-radius: 8px;">
                                                        <span style="color: #16a34a;">"✓"</span>
                                                        <div>
                                                            <div style="font-weight: 500;">"Task created successfully"</div>
                                                            <div style="color: var(--text-muted); font-size: 11px;">"Today, 08:00 AM"</div>
                                                        </div>
                                                    </div>
                                                    <div style="display: flex; gap: 10px; align-items: center; padding: 8px; background: var(--bg-hover); border-radius: 8px;">
                                                        <span style="color: var(--primary);">"✎"</span>
                                                        <div>
                                                            <div style="font-weight: 500;">"Notes and subtasks updated"</div>
                                                            <div style="color: var(--text-muted); font-size: 11px;">"Today, 09:30 AM"</div>
                                                        </div>
                                                    </div>
                                                </div>
                                                <button class="google-login-btn" on:click=move |_| show_activity_modal.1.set(false)>"Close"</button>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}

                            // Add Tag Modal
                            {let task_id_tags = task_id_tags.clone();
                            move || {
                                let task_id_tags = task_id_tags.clone();
                                if show_add_tag_modal.0.get() {
                                    view! {
                                        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                                            <div style="background: var(--bg-card); padding: 30px; border-radius: var(--radius); width: 380px; border: 1px solid var(--border-color); box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 20px;">
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <h3 style="font-size: 16px; font-weight: 700;">"Add Tag"</h3>
                                                    <button class="close-btn" on:click=move |_| show_add_tag_modal.1.set(false)>"×</"</button>
                                                </div>
                                                <input
                                                    type="text"
                                                    placeholder="Tag name..."
                                                    class="task-input"
                                                    style="padding: 10px; font-size: 14px;"
                                                    prop:value=new_tag_val.0
                                                    on:input=move |ev| new_tag_val.1.set(event_target_value(&ev))
                                                />
                                                <div style="display: flex; gap: 10px; justify-content: flex-end;">
                                                    <button style="background: transparent; border: 1px solid var(--border-color); padding: 8px 16px; border-radius: 8px; cursor: pointer;" on:click=move |_| show_add_tag_modal.1.set(false)>"Cancel"</button>
                                                    <button class="add-task-btn" style="padding: 8px 16px;" on:click=move |_| {
                                                        let trimmed = new_tag_val.0.get().trim().to_string();
                                                        if !trimmed.is_empty() {
                                                            let id = task_id_tags.clone();
                                                            tasks.1.update(|ts| {
                                                                if let Some(t) = ts.iter_mut().find(|x| x.id == id) {
                                                                    if !t.tags.contains(&trimmed) {
                                                                        t.tags.push(trimmed);
                                                                    }
                                                                }
                                                                if let Some(window) = web_sys::window() {
                                                                    if let Ok(Some(storage)) = window.local_storage() {
                                                                        if let Ok(json) = serde_json::to_string(ts) {
                                                                            let _ = storage.set_item("majeh_tasks", &json);
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                            let current_tasks = tasks.0.get();
                                                            if let Some(updated) = current_tasks.iter().find(|x| x.id == id) {
                                                                selected_task.1.set(Some(updated.clone()));
                                                            }
                                                            new_tag_val.1.set(String::new());
                                                            show_add_tag_modal.1.set(false);
                                                        }
                                                    }>"Add Tag"</button>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}
                        </aside>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}

            // Mobile Bottom Navigation Bar
            <nav class="mobile-bottom-nav">
                <>
                    <button class=move || if active_tab.0.get() == "active" { "mobile-nav-item active" } else { "mobile-nav-item" } on:click=move |_| set_tab("active")>
                        <span class="icon">"🏠"</span>
                        <span>"Tasks"</span>
                    </button>
                    <button class="mobile-nav-item" on:click=move |_| trigger_sync(())>
                        <span class="icon">"🔄"</span>
                        <span>"Sync"</span>
                    </button>
                    <button class="mobile-nav-item" on:click=move |_| export_tasks(())>
                        <span class="icon">"↕️"</span>
                        <span>"Import/Export"</span>
                    </button>
                    <button class="mobile-nav-item" on:click=move |_| show_settings.1.set(true)>
                        <span class="icon">"⚙️"</span>
                        <span>"Settings"</span>
                    </button>
                </>
            </nav>

            // Mobile Floating Action Button (FAB)
            <button class="mobile-fab" title="Add Task" on:click=move |_| {
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        if let Some(el) = doc.get_element_by_id("main-task-input") {
                            let _ = el.dyn_into::<web_sys::HtmlInputElement>().map(|i| i.focus());
                        }
                    }
                }
            }>
                "+"
            </button>
        </div>
    }
}


