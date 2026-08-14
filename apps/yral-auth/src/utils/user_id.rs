//! User ID generation utilities.
//!
//! User IDs are OAuth `sub` strings (for Google/Apple) or UUIDs (for
//! phone/WhatsApp auth and backend service accounts). No IC principals
//! or secp256k1 keys are involved.

use uuid::Uuid;

/// Generate a new random user ID (UUID v4).
pub fn generate_user_id() -> String {
    Uuid::new_v4().to_string()
}