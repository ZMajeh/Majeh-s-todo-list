/// Google OAuth Client ID (public, can be embedded in the app)
pub const GOOGLE_CLIENT_ID: &str = "816837125654-o1gp038ssrna485u40skhlbcgk4ppivt";

/// OAuth 2.0 endpoints
pub const OAUTH_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const OAUTH_REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// OAuth scopes for Todo application
pub const DEFAULT_OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/userinfo.email";

/// Redirect URI for OAuth callback (legacy constant, use get_oauth_redirect_uri() instead)
pub const OAUTH_REDIRECT_URI: &str = "http://localhost:8080/auth/callback/google";

/// Get redirect URI for OAuth callback dynamically based on current window location
pub fn get_oauth_redirect_uri() -> String {
    if let Some(window) = web_sys::window() {
        if let Ok(origin) = window.location().origin() {
            if !origin.is_empty() {
                return format!("{}/auth/callback/google", origin);
            }
        }
    }
    OAUTH_REDIRECT_URI.to_string()
}

/// Storage keys
pub const TOKENS_STORAGE_KEY: &str = "google_tokens";
pub const CLIENT_SECRET_STORAGE_KEY: &str = "google_client_secret";
