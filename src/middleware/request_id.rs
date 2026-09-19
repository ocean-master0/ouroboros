/// Request ID middleware.
///
/// Ensures every request and response carries an `X-Request-Id` header
/// populated with a UUID v4 string (FR10).
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

pub fn set_request_id_layer() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::x_request_id(MakeRequestUuid)
}

pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::x_request_id()
}
