use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use launcher_core::{build_manifest, manifest_json};
use semver::Version;

#[derive(Debug, Parser)]
#[command(about = "Generate a deterministic Voxtera Preview release manifest")]
struct Args {
    #[arg(long, value_name = "DIST")]
    input: PathBuf,
    #[arg(long, value_name = "Voxtera-windows-x64.zip")]
    archive: PathBuf,
    #[arg(long, value_name = "vX.Y.Z", value_parser = parse_version)]
    version: Version,
    #[arg(long, value_name = "VERSION", value_parser = parse_version)]
    minimum_launcher_version: Version,
    #[arg(long, value_name = "voxtera-manifest.json")]
    output: PathBuf,
}

fn parse_version(raw: &str) -> Result<Version, String> {
    Version::parse(raw.strip_prefix('v').unwrap_or(raw))
        .map_err(|_| "expected a semantic version".to_owned())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("voxtera-manifest: {error}");
            ExitCode::FAILURE
        },
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = build_manifest(
        &args.input,
        &args.archive,
        args.version,
        args.minimum_launcher_version,
    )?;
    std::fs::write(args.output, manifest_json(&manifest)?)?;
    Ok(())
}
