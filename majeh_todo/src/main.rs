use leptos::prelude::*;
use leptos_router::components::{Router, Routes, Route};
use leptos_router::path;
mod api;
mod components;
mod pages;

use pages::login::LoginPage;
use pages::dashboard::DashboardPage;
use pages::auth_callback::AuthCallbackPage;

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "Page not found">
                <Route path=path!("/login") view=LoginPage />
                <Route path=path!("/auth/callback/google") view=AuthCallbackPage />
                <Route path=path!("/") view=DashboardPage />
            </Routes>
        </Router>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App /> });
}
