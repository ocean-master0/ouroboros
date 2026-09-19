/// Request body size limit middleware.
///
/// Prevents memory exhaustion attacks by limiting body size to configured bytes (FR13).
use tower_http::limit::RequestBodyLimitLayer;

pub fn body_limit_layer(bytes: usize) -> RequestBodyLimitLayer {
    RequestBodyLimitLayer::new(bytes)
}
