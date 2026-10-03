//! Desktop builds: a plain `cargo run` of the app's binary.

use crate::project::{Project, cargo};
use crate::util::run;
use anyhow::{Context, Result};
use std::process::Command;

fn cargo_cmd(project: &Project, subcommand: &str, release: bool) -> Result<Command> {
    let bin = project
        .bin_name
        .as_deref()
        .context("desktop needs a binary target: add src/main.rs calling `<crate>::run()`")?;
    let mut cmd = Command::new(cargo());
    cmd.current_dir(&project.crate_dir)
        .args([subcommand, "-p", &project.package, "--bin", bin])
        // Read by `reactive::app!` at compile time (the GTK application ID).
        .env("REACTIVE_APP_ID", &project.identifier);
    if release {
        cmd.arg("--release");
    }
    Ok(cmd)
}

pub fn cargo_build(project: &Project, release: bool) -> Result<()> {
    run(&mut cargo_cmd(project, "build", release)?)
}

pub fn cargo_run(project: &Project, release: bool, args: &[String]) -> Result<()> {
    let mut cmd = cargo_cmd(project, "run", release)?;
    if !args.is_empty() {
        cmd.arg("--").args(args);
    }
    run(&mut cmd)
}
