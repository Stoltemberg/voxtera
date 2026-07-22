use std::{
    io,
    path::{Path, PathBuf},
    process::ExitCode,
};

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
    ensure_output_outside_input(&args.input, &args.output)?;
    let manifest = build_manifest(
        &args.input,
        &args.archive,
        args.version,
        args.minimum_launcher_version,
    )?;
    std::fs::write(args.output, manifest_json(&manifest)?)?;
    Ok(())
}

fn ensure_output_outside_input(input: &Path, output: &Path) -> io::Result<()> {
    let input = input.canonicalize()?;
    let output_parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let existing_output = output.canonicalize().ok();
    if output_parent.starts_with(&input)
        || existing_output
            .as_ref()
            .is_some_and(|path| path.starts_with(&input))
    {
        return Err(io::Error::other(
            "O manifesto de saída deve ficar fora da distribuição de entrada.",
        ));
    }
    Ok(())
}
