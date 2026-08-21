// Convenience re-export of the crate-level icon functions.
//
// The canonical icon implementations live in `crate::icons` (logos, twemoji,
// flat-color sets). This module re-exports them under `crate::components::icons`
// so component code can import icons from the components namespace when
// convenient.

pub use crate::icons::logos;
pub use crate::icons::twemoji;
pub use crate::icons::flat_color;