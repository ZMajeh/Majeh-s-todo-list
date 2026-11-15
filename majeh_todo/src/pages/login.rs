use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use crate::api::auth::{GOOGLE_CLIENT_ID, OAUTH_AUTHORIZE_URL, OAUTH_REDIRECT_URI, DEFAULT_OAUTH_SCOPES};

#[component]
pub fn LoginPage() -> impl IntoView {
    let navigate = use_navigate();
    let client_secret = signal(String::new());

    let handle_login = move |_| {
        let secret = client_secret.0.get();
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if !secret.is_empty() {
                    let _ = storage.set_item("google_client_secret", &secret);
                }
            }
        }

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            OAUTH_AUTHORIZE_URL,
            GOOGLE_CLIENT_ID,
            encode_uri(OAUTH_REDIRECT_URI),
            encode_uri(DEFAULT_OAUTH_SCOPES)
        );

        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href(&auth_url);
        }
    };

    let bypass_login = move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("user_email", "local@user.com");
                let _ = storage.set_item("user_name", "Local User");
            }
        }
        navigate("/", Default::default());
    };

    view! {
        <div class="login-page">
            <div class="login-container">
                <h2>"Majeh's TODO LIST"</h2>
                <p>"Sign in with Google to sync your tasks across devices."</p>

                <div style="width: 100%; display: flex; flex-direction: column; gap: 8px;">
                    <label style="font-size: 13px; color: var(--text-muted);">"Google Client Secret (Optional)"</label>
                    <input
                        type="password"
                        placeholder="Enter Client Secret"
                        on:input=move |ev| client_secret.1.set(event_target_value(&ev))
                        prop:value=client_secret.0
                        class="task-input"
                    />
                </div>

                <button
                    class="google-login-btn"
                    on:click=handle_login
                >
                    "Login with Google"
                </button>
                <button
                    class="nav-item"
                    style="justify-content: center; background: var(--bg-hover);"
                    on:click=bypass_login
                >
                    "Continue in Local Mode"
                </button>
            </div>
        </div>
    }
}

fn encode_uri(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_else(|| s.to_string())
}
