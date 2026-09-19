/// Localhost-only access control middleware.
///
/// Intended for `/internal/*` endpoints (AC-28, full-release requirement).
/// In MVP builds, this is scaffolded per architecture_doc.md §5.5.
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn localhost_only_middleware(req: Request, next: Next) -> Response {
    next.run(req).await
}
