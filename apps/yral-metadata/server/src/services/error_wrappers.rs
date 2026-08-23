use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// OpenAPI response wrapper structs used in #[utoipa::path] response annotations
// (body = ErrorWrapper<T>, body = OkWrapper<T>, body = NullOk).

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct ErrorWrapper<T: ToSchema> {
    err: T,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct OkWrapper<T: ToSchema> {
    ok: T,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct NullOk {
    ok: (),
}

// Error detail structs used in #[schema(value_type = ...)] annotations on the
// Error enum variants in utils/error.rs. These are OpenAPI schema shapes only —
// they describe the API documentation representation, not runtime conversion.
// The fields carry #[schema(example = ...)] so the generated Swagger docs show
// realistic example values.

#[derive(Debug, ToSchema, Serialize)]
pub struct ConfigErrorDetail {
    #[schema(example = "Frozen")]
    pub kind: String,
    #[schema(example = "Configuration is frozen and no further mutations can be made.")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct IdentityErrorDetail {
    #[schema(example = "Signature verification failed")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SerdeJsonErrorDetail {
    #[schema(example = 1)]
    pub line: usize,
    #[schema(example = 1)]
    pub column: usize,
    #[schema(example = "EOF while parsing a value")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct JwtErrorDetail {
    #[schema(example = "InvalidToken")]
    pub kind: String,
    #[schema(example = "Expired token")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct VarErrorDetail {
    #[schema(example = "NotPresent")]
    pub kind: String,
    #[schema(example = "Environment variable not present, or not unicode")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct PrincipalErrorDetail {
    #[schema(example = "BytesTooLong")]
    pub kind: String,
    #[schema(example = "Bytes is longer than 29 bytes.")]
    pub message: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub struct AgentErrorDetail {
    #[schema(example = "InvalidReplicaUrl")]
    pub kind: String,
    #[schema(example = "Invalid Replica URL: \"https://replica.example.com\"")]
    pub message: String,
}

/// OpenAPI schema shape for `std::io::Error` — used in
/// `#[schema(value_type = IOErrorData)]` on `Error::IO`. Represents the
/// kind and message of an IO error in the API documentation.
#[derive(Debug, ToSchema, Serialize)]
pub struct IOErrorData {
    #[schema(example = "Os(\"Invalid OS error\")")]
    pub message: String,
}
