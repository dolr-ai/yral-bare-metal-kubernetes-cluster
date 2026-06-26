use anyhow::Result;
use std::process::Command;

// cargo test -p task_runner local_server_start -- --ignored
#[test]
#[ignore = "Use this to run the local development workflow"]
fn local_server_start() -> Result<()> {
    Command::new("spacetime").arg("start").status()?;

    Ok(())
}

#[test]
fn check_endpoint() -> Result<()> {
    let response = reqwest::blocking::get("http://localhost:3000/v1/identity")?.text()?;

    println!("{:?}", response);
    Ok(())
}
