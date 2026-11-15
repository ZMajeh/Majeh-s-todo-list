use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::hooks::use_query_map;
use crate::api::auth::{GOOGLE_CLIENT_ID, OAUTH_TOKEN_URL, get_oauth_redirect_uri};
use gloo_net::http::Request;

#[component]
pub fn AuthCallbackPage() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    let status_message = signal("Completing sign-in with Google...".to_string());

    Effect::new(move |_| {
        let params = query.get();
        if let Some(code) = params.get("code") {
            let code = code.to_string();
            let navigate_c = navigate.clone();
            let status_message_c = status_message.1;

            wasm_bindgen_futures::spawn_local(async move {
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        navigate_c("/", Default::default());
                        return;
                    }
                };

                let storage = window.local_storage().ok().flatten();
                let client_secret = storage
                    .as_ref()
                    .and_then(|s| s.get_item("google_client_secret").ok().flatten())
                    .unwrap_or_default();

                status_message_c.set("Exchanging authorization code...".to_string());

                let body = format!(
                    "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
                    encode_uri(&code),
                    encode_uri(GOOGLE_CLIENT_ID),
                    encode_uri(&client_secret),
                    encode_uri(&get_oauth_redirect_uri())
                );

                let token_res = match Request::post(OAUTH_TOKEN_URL)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(body) {
                    Ok(req) => req.send().await,
                    Err(e) => Err(e.into()),
                };

                if let Ok(resp) = token_res {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) {
                            if let Some(ref s) = storage {
                                let _ = s.set_item("google_access_token", access_token);
                            }

                            status_message_c.set("Fetching user profile...".to_string());

                            let userinfo_res = Request::get("https://www.googleapis.com/oauth2/v3/userinfo")
                                .header("Authorization", &format!("Bearer {}", access_token))
                                .send()
                                .await;

                            if let Ok(ui_resp) = userinfo_res {
                                if let Ok(ui_json) = ui_resp.json::<serde_json::Value>().await {
                                    if let Some(email) = ui_json.get("email").and_then(|v| v.as_str()) {
                                        if let Some(ref s) = storage {
                                            let _ = s.set_item("user_email", email);
                                        }
                                    }
                                    if let Some(name) = ui_json.get("name").and_then(|v| v.as_str()) {
                                        if let Some(ref s) = storage {
                                            let _ = s.set_item("user_name", name);
                                        }
                                    } else if let Some(email) = ui_json.get("email").and_then(|v| v.as_str()) {
                                        let username = derive_username_from_email(email);
                                        if let Some(ref s) = storage {
                                            let _ = s.set_item("user_name", &username);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallbacks if not set
                if let Some(ref s) = storage {
                    if s.get_item("user_email").ok().flatten().is_none() {
                        let _ = s.set_item("user_email", "user@gmail.com");
                    }
                    if s.get_item("user_name").ok().flatten().is_none() {
                        let _ = s.set_item("user_name", "Google User");
                    }
                }

                navigate_c("/", Default::default());
            });
        } else if let Some(error) = params.get("error") {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!("OAuth error: {}", error)));
            navigate("/login", Default::default());
        }
    });

    view! {
        <div class="login-page">
            <div class="login-container">
                <h2>"Authenticating..."</h2>
                <p>{move || status_message.0.get()}</p>
                <div class="loading-spinner"></div>
            </div>
        </div>
    }
}

fn derive_username_from_email(email: &str) -> String {
    let parts: Vec<&str> = email.split('@').collect();
    if let Some(local_part) = parts.first() {
        let cleaned = local_part.replace(['.', '_', '-'], " ");
        let capitalized: String = cleaned
            .split_whitespace()
            .map(|word| {
                let mut c = word.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ");
        if !capitalized.is_empty() {
            return capitalized;
        }
    }
    "Google User".to_string()
}

fn encode_uri(s: &str) -> String {
    js_sys::encode_uri_component(s).as_string().unwrap_or_else(|| s.to_string())
}
