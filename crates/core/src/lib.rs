//! Recall Gate core library.
//!
//! Domain types and FSRS scheduling land in `pr-core` per `docs/domain.md`.

/// Smoke hook so CI and `cargo test` prove the workspace layout before domain code exists.
pub fn workspace_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_workspace_ready() {
        assert!(super::workspace_ready());
    }
}
