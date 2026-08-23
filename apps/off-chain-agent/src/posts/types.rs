use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for post operations. The user is authenticated via JWT
/// Bearer token in the `Authorization` header (verified by middleware),
/// so the body no longer carries `delegated_identity_wire`.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PostRequest<T> {
    pub post_id: String,
    #[serde(flatten)]
    pub request_body: T,
}
