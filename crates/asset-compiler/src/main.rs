use anyhow::{bail, Context, Result};
use asset_compiler::bake;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, kind, dir] if command == "bake" => {
            bake_asset(kind, Path::new(dir), None)?;
        }
        [command, kind, dir, out] if command == "bake" => {
            bake_asset(kind, Path::new(dir), Some(Path::new(out)))?;
        }
        [command, glb] if command == "bake-glb" => {
            bake_glb(Path::new(glb), None)?;
        }
        [command, glb, out] if command == "bake-glb" => {
            bake_glb(Path::new(glb), Some(Path::new(out)))?;
        }
        [command, dir] if command == "bake-glb-track" => {
            bake_glb_track(Path::new(dir), None)?;
        }
        [command, dir, out] if command == "bake-glb-track" => {
            bake_glb_track(Path::new(dir), Some(Path::new(out)))?;
        }
        [command, dir] if command == "bake-ac-track" => {
            bake_ac_track(Path::new(dir), None, 1.0, None)?;
        }
        [command, dir, flag, value] if command == "bake-ac-track" && *flag == "--scale" => {
            bake_ac_track(Path::new(dir), None, parse_scale(value)?, None)?;
        }
        [command, dir, flag, value]
            if command == "bake-ac-track" && *flag == "--texture-overrides" =>
        {
            bake_ac_track(Path::new(dir), None, 1.0, Some(Path::new(value)))?;
        }
        [command, dir, out] if command == "bake-ac-track" => {
            bake_ac_track(Path::new(dir), Some(Path::new(out)), 1.0, None)?;
        }
        [command, dir, out, flag, value] if command == "bake-ac-track" && *flag == "--scale" => {
            bake_ac_track(
                Path::new(dir),
                Some(Path::new(out)),
                parse_scale(value)?,
                None,
            )?;
        }
        [command, dir, out, flag, value]
            if command == "bake-ac-track" && *flag == "--texture-overrides" =>
        {
            bake_ac_track(
                Path::new(dir),
                Some(Path::new(out)),
                1.0,
                Some(Path::new(value)),
            )?;
        }
        [command, pack] if command == "inspect" => {
            let bytes = read_pack(pack)?;
            bake::inspect(&bytes)?;
        }
        [command, pack] if command == "validate" => {
            let bytes = read_pack(pack)?;
            bake::validate(&bytes)?;
        }
        _ => {
            usage();
            bail!("invalid arguments");
        }
    }
    Ok(())
}

fn parse_scale(value: &str) -> Result<f32> {
    let scale: f32 = value
        .parse()
        .with_context(|| format!("invalid track scale {value}"))?;
    if !scale.is_finite() || scale <= 0.0 {
        bail!("track scale must be a positive finite number, got {scale}");
    }
    Ok(scale)
}

fn bake_glb_track(dir: &Path, out: Option<&Path>) -> Result<()> {
    let bytes = bake::bake_glb_track(dir)?;
    let output = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.join("pack.sima"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, &bytes).with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "baked GLB track {} -> {} ({} bytes)",
        dir.display(),
        output.display(),
        bytes.len()
    );
    Ok(())
}

fn bake_ac_track(
    dir: &Path,
    out: Option<&Path>,
    scale: f32,
    overrides: Option<&Path>,
) -> Result<()> {
    let bytes = bake::bake_ac_track_with_overrides(dir, scale, overrides)?;
    let output = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.join("pack.sima"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, &bytes).with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "baked AC track {} -> {} at scale {scale} ({} bytes)",
        dir.display(),
        output.display(),
        bytes.len()
    );
    Ok(())
}

fn bake_asset(kind: &str, dir: &Path, out: Option<&Path>) -> Result<()> {
    let bytes = match kind {
        "car" => bake::bake_car(dir)?,
        "track" => bake::bake_track(dir)?,
        other => bail!("unknown asset kind {other}; expected car or track"),
    };
    let output = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.join("pack.sima"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, &bytes).with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "baked {} -> {} ({} bytes)",
        dir.display(),
        output.display(),
        bytes.len()
    );
    Ok(())
}

fn bake_glb(input: &Path, out: Option<&Path>) -> Result<()> {
    let bytes = bake::bake_car_glb(input)?;
    let output = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("sima"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output, &bytes).with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "baked GLB {} -> {} ({} bytes)",
        input.display(),
        output.display(),
        bytes.len()
    );
    Ok(())
}

fn read_pack(path: &str) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("failed to read {path}"))
}

fn usage() {
    eprintln!(
        "usage:\n  asset-compiler bake <car|track> <dir> [out]\n  asset-compiler bake-glb <car.glb> [out]\n  asset-compiler bake-glb-track <dir> [out]\n  asset-compiler bake-ac-track <dir> [out] [--scale <factor>] [--texture-overrides <file>]\n  asset-compiler inspect <pack>\n  asset-compiler validate <pack>"
    );
}
