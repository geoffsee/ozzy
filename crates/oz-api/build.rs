#[path = "src/openapi.rs"]
mod openapi;

use std::{env, fs, path::Path, path::PathBuf, process::Command};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("failed to resolve workspace root from crate path");
    let web_dir = workspace_root.join("apps/web");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=BUN_PUBLIC_TELEMETRY_ENDPOINT");
    println!("cargo:rerun-if-env-changed=TELEMETRY_SINK_URL");
    println!("cargo:rerun-if-env-changed=OZ_TELEMETRY_ENDPOINT");
    for file in ["build.ts", "package.json", "bun.lock", "bunfig.toml"] {
        println!("cargo:rerun-if-changed={}", web_dir.join(file).display());
    }
    emit_rerun_for_dir(&web_dir.join("src"));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let output_file = out_dir.join("index.html");

    let mut build_cmd = Command::new("bun");
    build_cmd
        .args([
            "run",
            "build",
            "--outfile",
            output_file
                .to_str()
                .expect("single-file output path contains invalid UTF-8"),
        ])
        .current_dir(&web_dir);

    if let Some(endpoint) = resolve_telemetry_events_endpoint() {
        build_cmd.env("BUN_PUBLIC_TELEMETRY_ENDPOINT", endpoint);
    }

    if let Ok(sink_url) = env::var("TELEMETRY_SINK_URL") {
        build_cmd.env("TELEMETRY_SINK_URL", sink_url);
    }

    let status = build_cmd
        .status()
        .expect("failed to execute bun build for oz web client");

    if !status.success() {
        panic!("bun web build failed while compiling oz-api");
    }

    let spec = openapi::build_openapi();
    let spec_json = serde_json::to_string_pretty(&spec)
        .expect("failed to serialize generated OpenAPI spec to JSON");
    let openapi_path = resolve_openapi_path(&workspace_root);
    if let Some(parent) = openapi_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("failed to create OpenAPI output dir {}: {error}", parent.display()));
    }
    fs::write(&openapi_path, spec_json).unwrap_or_else(|error| {
        panic!("failed to write generated OpenAPI spec to {}: {error}", openapi_path.display())
    });

    println!("cargo:warning=generated OpenAPI spec at {}", openapi_path.display());
    println!("cargo:rustc-env=OZ_UI_HTML_PATH={}", output_file.display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("src/openapi.rs").display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src/routes/mod.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("crates/oz-core/src/models.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root.join("crates/oz-core/src/lib.rs").display()
    );
}

fn emit_rerun_for_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }

    println!("cargo:rerun-if-changed={}", dir.display());

    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read directory {}: {error}", dir.display());
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to read directory entry in {}: {error}",
                dir.display()
            );
        });
        let path = entry.path();
        if path.is_dir() {
            emit_rerun_for_dir(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn resolve_openapi_path(workspace_root: &Path) -> PathBuf {
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));

    let target_dir = if target_dir.is_absolute() {
        target_dir
    } else {
        workspace_root.join(target_dir)
    };

    target_dir.join("openapi").join("openapi.json")
}

fn resolve_telemetry_events_endpoint() -> Option<String> {
    env::var("BUN_PUBLIC_TELEMETRY_ENDPOINT")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("TELEMETRY_SINK_URL")
                .ok()
                .filter(|value| !value.is_empty())
                .map(|sink| format!("{}/v1/events", sink.trim_end_matches('/')))
        })
        .or_else(|| {
            env::var("OZ_TELEMETRY_ENDPOINT")
                .ok()
                .filter(|value| !value.is_empty())
        })
}
