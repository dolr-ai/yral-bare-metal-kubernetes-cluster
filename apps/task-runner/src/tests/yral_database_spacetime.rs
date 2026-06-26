use anyhow::Result;
use std::process::Command;

// cargo test -p task_runner local_server_start -- --ignored --nocapture
#[test]
#[ignore = "Use this to run the local development workflow"]
fn local_server_start() -> Result<()> {
    Command::new("spacetime").arg("start").status()?;

    Ok(())
}
