mod android;
mod apple;
mod desktop;
mod doctor;
mod new;
mod project;
mod util;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use project::Project;
use std::path::PathBuf;

/// Cargo invokes `cargo-reactive reactive <args>`.
#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    Reactive(Cli),
}

#[derive(Args)]
#[command(
    version,
    about = "Create, build and run reactive apps on every platform"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new app.
    New {
        /// Directory to create; its name becomes the crate name.
        path: PathBuf,
        /// Display name. Defaults to the crate name in title case.
        #[arg(long)]
        name: Option<String>,
        /// Reverse-DNS app ID. Defaults to `com.example.<crate>`.
        #[arg(long)]
        identifier: Option<String>,
        /// Depend on a local framework checkout at this path.
        #[arg(long)]
        reactive_path: Option<PathBuf>,
    },
    /// Build and launch the app.
    Run {
        #[command(flatten)]
        build: BuildArgs,
        /// Arguments passed to the app (desktop only).
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Build the app without launching it.
    Build {
        #[command(flatten)]
        build: BuildArgs,
    },
    /// Regenerate a platform project, e.g. to open it in Android Studio.
    Generate {
        platform: MobilePlatform,
        #[command(flatten)]
        package: PackageArgs,
        #[arg(long)]
        use_maven: bool,
    },
    /// Open the platform project in its IDE.
    Open {
        platform: MobilePlatform,
        #[command(flatten)]
        package: PackageArgs,
    },
    /// Take ownership of a platform project: it moves to `<crate>/<platform>`
    /// and is no longer regenerated.
    Eject {
        platform: MobilePlatform,
        #[command(flatten)]
        package: PackageArgs,
        #[arg(long)]
        use_maven: bool,
    },
    /// Check the toolchains every target needs.
    Doctor,
}

#[derive(Args)]
struct PackageArgs {
    /// The app package, when run from a workspace root.
    #[arg(short, long)]
    package: Option<String>,
    #[arg(long)]
    manifest_path: Option<PathBuf>,
}

impl PackageArgs {
    fn load(&self) -> Result<Project> {
        Project::load(self.manifest_path.as_deref(), self.package.as_deref())
    }
}

#[derive(Args)]
struct BuildArgs {
    #[arg(short, long, value_enum, default_value_t = Target::Host)]
    target: Target,
    #[arg(short, long)]
    release: bool,
    /// Android device serial, or iOS simulator name or UDID.
    #[arg(short, long)]
    device: Option<String>,
    /// Use the published Gradle plugin and Android library even when the
    /// framework is a local checkout.
    #[arg(long)]
    use_maven: bool,
    #[command(flatten)]
    package: PackageArgs,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Target {
    /// This machine.
    Host,
    Linux,
    Macos,
    Android,
    Ios,
}

#[derive(Clone, Copy, ValueEnum)]
enum MobilePlatform {
    Android,
    Ios,
}

impl MobilePlatform {
    fn dir_name(self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Ios => "ios",
        }
    }
}

fn main() {
    let Cargo::Reactive(cli) = Cargo::parse();
    if let Err(err) = run(cli.command) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run(cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::New {
            path,
            name,
            identifier,
            reactive_path,
        } => new::new(new::Options {
            path,
            name,
            identifier,
            reactive_path,
        }),
        Cmd::Run { build, args } => {
            let project = build.package.load()?;
            match resolve(build.target)? {
                Target::Linux => desktop::cargo_run(&project, build.release, &args),
                Target::Macos => apple::run_macos(&project, build.release),
                Target::Android => android::run_app(&project, &android_opts(&build)),
                Target::Ios => apple::run_ios(&project, &ios_opts(&build)),
                Target::Host => unreachable!(),
            }
        }
        Cmd::Build { build } => {
            let project = build.package.load()?;
            match resolve(build.target)? {
                Target::Linux => desktop::cargo_build(&project, build.release),
                Target::Macos => apple::bundle_macos(&project, build.release).map(drop),
                Target::Android => android::build(&project, &android_opts(&build)),
                Target::Ios => apple::build_ios(&project, &ios_opts(&build)),
                Target::Host => unreachable!(),
            }
        }
        Cmd::Generate {
            platform,
            package,
            use_maven,
        } => {
            let project = package.load()?;
            match platform {
                MobilePlatform::Android => {
                    let dir = android::prepare(
                        &project,
                        &android::Options {
                            release: false,
                            use_maven,
                            device: None,
                        },
                    )?;
                    util::status("Ready", dir.display());
                }
                MobilePlatform::Ios => {
                    let dir = apple::prepare_ios(&project)?;
                    util::status("Ready", dir.display());
                }
            }
            Ok(())
        }
        Cmd::Open { platform, package } => {
            let project = package.load()?;
            match platform {
                MobilePlatform::Ios => apple::open_ios(&project),
                MobilePlatform::Android => {
                    let opts = android::Options {
                        release: false,
                        use_maven: false,
                        device: None,
                    };
                    let dir = android::prepare(&project, &opts)?;
                    let studio = if cfg!(target_os = "macos") {
                        std::process::Command::new("open")
                            .args(["-a", "Android Studio"])
                            .arg(&dir)
                            .status()
                    } else {
                        std::process::Command::new("studio").arg(&dir).status()
                    };
                    if !studio.is_ok_and(|s| s.success()) {
                        util::status("Open", format!("{} in Android Studio", dir.display()));
                    }
                    Ok(())
                }
            }
        }
        Cmd::Eject {
            platform,
            package,
            use_maven,
        } => {
            let project = package.load()?;
            let (dir, ejected) = project.platform_dir(platform.dir_name());
            if ejected {
                bail!("{} is already ejected", dir.display());
            }
            let dest = project.crate_dir.join(platform.dir_name());
            match platform {
                MobilePlatform::Android => android::generate(&project, &dest, use_maven)?,
                MobilePlatform::Ios => apple::generate_ios(&project, &dest)?,
            }
            let _ = std::fs::remove_dir_all(&dir);
            util::status(
                "Ejected",
                format!(
                    "{} is yours now: commit it, and edit it freely. \
                     [package.metadata.reactive] no longer applies to it.",
                    dest.display()
                ),
            );
            Ok(())
        }
        Cmd::Doctor => {
            if !doctor::doctor() {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

/// Maps `host` to this machine and rejects targets this machine can't build.
fn resolve(target: Target) -> Result<Target> {
    let host = if cfg!(target_os = "macos") {
        Target::Macos
    } else if cfg!(target_os = "linux") {
        Target::Linux
    } else {
        bail!("this OS has no desktop backend; use --target android");
    };
    match target {
        Target::Host => Ok(host),
        Target::Ios if host != Target::Macos => bail!("iOS builds need macOS with Xcode"),
        Target::Macos | Target::Linux if target != host => {
            bail!("desktop apps build for the machine they run on; use --target host")
        }
        t => Ok(t),
    }
}

fn android_opts(build: &BuildArgs) -> android::Options {
    android::Options {
        release: build.release,
        use_maven: build.use_maven,
        device: build.device.clone(),
    }
}

fn ios_opts(build: &BuildArgs) -> apple::IosOptions {
    apple::IosOptions {
        release: build.release,
        device: build.device.clone(),
    }
}
