use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SessionType {
    AnonymousSession,
    RegisteredSession,
}
