use anyhow::{Ok, Result};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

static SERVER: OnceLock<std::process::Child> = OnceLock::new();

// cargo test -p task_runner local_server_start -- --ignored
#[test]
#[ignore = "Use this to run the local development workflow"]
fn local_server_start() -> Result<()> {
    let child = Command::new("spacetime")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    SERVER.set(child).expect("server already started");

    // Give the server time to start
    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

#[test]
#[ignore = "Check if the server is up"]
fn check_endpoint() -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client.post("http://localhost:3000/v1/identity").send()?;

    println!("{:?}", response.text());
    Ok(())
}

#[test]
#[ignore = "Run this after check_endpoint to clean up the server process"]
fn cleanup_server() -> Result<()> {
    Command::new("pkill")
        .arg("-f")
        .arg("spacetime")
        .spawn()?
        .wait()?;
    Ok(())
}

#[test]
#[ignore = "Publish locally"]
fn publish_locally() -> Result<()> {
    Command::new("spacetime")
        .arg("publish")
        .arg("--env")
        .arg("dev")
        .spawn()?
        .wait()?;

    Ok(())
}

#[test]
#[ignore = "Publish to maincloud"]
fn publish_to_maincloud() -> Result<()> {
    Command::new("spacetime").arg("publish").spawn()?.wait()?;

    Ok(())
}
