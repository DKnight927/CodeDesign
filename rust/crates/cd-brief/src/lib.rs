//! cd-brief — Brief data type + session state machine + image_understand.
//!
//! This crate owns the session-level glue between the Interpreter's
//! structured Brief, the Planner's DesignPlan, and the Critic's
//! findings. It is the seam the CLI drives.

pub mod brief;
pub mod state;
pub mod vision;

pub use brief::{Brief, ClarifyAsk, ClarifyAnswer, Platform};
pub use state::{Phase, Session, Transition};
pub use vision::{VlClient, VlConfig, VlError};

pub const CRATE_NAME: &str = "cd-brief";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_name_matches() {
        assert_eq!(CRATE_NAME, "cd-brief");
    }
}
