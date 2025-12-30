// API Routes untuk Chat Service dengan JWT-Only architecture

use crate::config::AppState;
use crate::handlers::{conversations, messages, upload, websocket};
use crate::middleware::{auth::jwt_auth_middleware, rate_limit::rate_limit_middleware};
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    routing::{delete, get, post},
    Router,
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

// OpenAPI Documentation untuk Chat Service
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Chat Service API",
        version = "1.0.0",
        description = "Real-time Chat Service untuk Big Auto vehicle marketplace\n\n## Features\n\n- 💬 Real-time messaging dengan WebSocket\n- 📁 File & media upload support\n- 🔍 Message search & filtering\n- 👥 Conversation management\n- 📱 Typing indicators & read receipts\n- 🔒 JWT-Only authentication (no CSRF required)\n- 🌐 Redis-based rate limiting",
    ),
    paths(
        conversations::create_conversation,
        conversations::get_user_conversations,
        conversations::get_conversation_by_id,
        conversations::get_conversation_with_details,
        conversations::mark_conversation_read,
        conversations::get_unread_count,
        conversations::health_check,
        messages::send_message,
        messages::send_typing_indicator,
        messages::get_conversation_messages,
        messages::get_latest_message,
        messages::get_message_count,
        messages::search_messages,
        messages::get_message_by_id,
        messages::mark_message_read,
        messages::delete_message,
        messages::get_unread_count,
        messages::get_media_messages,
        messages::get_messages_by_sender,
        messages::send_message_with_files,
        messages::generate_message_preview,
        upload::upload_file,
    ),
    components(
        schemas(
            crate::domain::Conversation,
            crate::domain::Message,
            crate::domain::CreateConversationRequest,
            crate::domain::CreateMessageRequest,
            crate::domain::MessageType,
            conversations::ConversationListResponse,
            conversations::ConversationWithDetailsResponse,
            crate::config::HealthCheckResponse,
            messages::MessageListResponse,
            messages::MessageCountResponse,
            upload::UploadResponse,
            upload::UploadedFile,
            upload::FileCategory,
            messages::CreateMessageWithFilesRequest,
            messages::MessagePreviewResponse,
            messages::TypingIndicatorRequest,
        )
    ),
    tags(
        (name = "chat-service", description = "Real-time chat service for Big Auto marketplace")
    ),
    modifiers(&SecurityAddon),
    servers(
        (url = "https://api.bigauto.com", description = "Production server")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub struct ApiDoc;

// Security scheme modifier untuk Bearer JWT authentication
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build()
                ),
            )
        }
    }
}

// Security headers middleware
async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';"
            .parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    headers.insert("Permissions-Policy", "camera=(), microphone=(), geolocation=()".parse().unwrap());
    headers.insert("Strict-Transport-Security", "max-age=31536000; includeSubDomains".parse().unwrap());

    response
}

// Buat router dengan JWT-only security dan Redis rate limiting
pub fn create_router(state: AppState) -> Router {
    if state.config.is_production() {
        tracing::warn!("Chat Service running in PRODUCTION mode");
    } else {
        tracing::info!("Chat Service running in DEVELOPMENT mode");
    }

    // CORS configuration dari environment
    let frontend_url = std::env::var("FRONTEND_URL")
        .expect("FRONTEND_URL environment variable HARUS diisi di .env file");

    let allowed_origin = frontend_url.parse::<axum::http::HeaderValue>()
        .expect("FRONTEND_URL harus valid URL format");

    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::CONTENT_TYPE,
        ])
        .allow_credentials(false) 
        .max_age(Duration::from_secs(86400));

    // Setup OpenAPI documentation
    let mut openapi = ApiDoc::openapi();
    SecurityAddon.modify(&mut openapi);

    // Public routes - tanpa JWT authentication
    let public_routes = Router::new()
        .route("/health", get(conversations::health_check))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi))
        .with_state(state.clone());

    // Protected API routes - dengan JWT authentication
    let protected_routes = build_api_routes(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ));

    // Combine semua routes dengan shared middleware
    public_routes
        .nest("/api", protected_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(axum::middleware::from_fn_with_state(
            state.rate_limiter.clone(),
            rate_limit_middleware,
        ))
}

// Build API routes dengan JWT authentication
fn build_api_routes(state: AppState) -> Router {
    Router::new()
        // ===== WebSocket Endpoint =====
        .route("/ws/chat/{conversation_id}", get(websocket::websocket_handler))

        // ===== Conversation Operations =====
        .route("/conversations", post(conversations::create_conversation))
        .route("/conversations/user/{user_id}", get(conversations::get_user_conversations))
        .route("/conversations/{conversation_id}", get(conversations::get_conversation_by_id))
        .route("/conversations/{conversation_id}/details", get(conversations::get_conversation_with_details))
        .route("/conversations/{conversation_id}/read", post(conversations::mark_conversation_read))
        .route("/conversations/unread", get(conversations::get_unread_count))

        // ===== Message Operations =====
        .route("/messages", post(messages::send_message))
        .route("/messages/typing", post(messages::send_typing_indicator))
        .route("/messages/conversation/{conversation_id}", get(messages::get_conversation_messages))
        .route("/messages/latest/{conversation_id}", get(messages::get_latest_message))
        .route("/messages/count/{conversation_id}", get(messages::get_message_count))
        .route("/messages/search", get(messages::search_messages))
        .route("/messages/{message_id}", get(messages::get_message_by_id))
        .route("/messages/{message_id}/read", post(messages::mark_message_read))
        .route("/messages/{message_id}", delete(messages::delete_message))
        .route("/messages/unread/{conversation_id}", get(messages::get_unread_count))
        .route("/messages/media/{conversation_id}", get(messages::get_media_messages))
        .route("/conversations/{conversation_id}/messages/sender/{sender_id}", get(messages::get_messages_by_sender))
        .route("/messages/with-files", post(messages::send_message_with_files))
        .route("/messages/preview", post(messages::generate_message_preview))

        // ===== File Upload Operations =====
        .route("/upload", post(upload::upload_file))

        .with_state(state)
}