/// Google OAuth Client ID (public, can be embedded in the app)
pub const GOOGLE_CLIENT_ID: &str = "816837125654-o1gp038ssrna485u40skhlbcgk4ppivt";

/// OAuth 2.0 endpoints
pub const OAUTH_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const OAUTH_REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// OAuth scopes for Todo application
pub const DEFAULT_OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/userinfo.email";

/// Redirect URI for OAuth callback
pub const OAUTH_REDIRECT_URI: &str = "http://localhost:8080/auth/callback/google";

/// Storage keys
pub const TOKENS_STORAGE_KEY: &str = "google_tokens";
pub const CLIENT_SECRET_STORAGE_KEY: &str = "google_client_secret";
