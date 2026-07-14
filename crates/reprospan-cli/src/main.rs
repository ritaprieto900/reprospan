use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use reprospan_core::{Bundle, Evaluation, Patch};
use reprospan_store::{Store, StoreError};
use serde::Serialize;

const BUNDLE_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/contracts/fixtures/v1/failed-tool-run.bundle.json"
));
const PATCH_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/contracts/fixtures/v1/fix-tool-result.patch.json"
));
const EVAL_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packages/contracts/fixtures/v1/fix-tool-result.eval.json"
));

#[derive(Parser)]
#[command(
    name = "reprospan",
    about = "Local-first replay debugging for AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Import {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
    },
    Timeline {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        bundle_id: String,
    },
    Export {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        bundle_id: String,
    },
    Patch {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        patch: PathBuf,
    },
    Diff {
        #[arg(long)]
        before: PathBuf,
        #[arg(long)]
        after: PathBuf,
    },
    Eval {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        eval: PathBuf,
    },
    Demo {
        #[arg(long)]
        db: PathBuf,
    },
    Serve {
        #[arg(long)]
        db: PathBuf,
        #[arg(long, default_value = "127.0.0.1:8787")]
        listen: SocketAddr,
    },
}

#[derive(Serialize)]
struct DemoSummary {
    bundle_id: String,
    imported: bool,
    exported: bool,
    patch_id: String,
    changed_event_count: usize,
    eval_id: String,
    eval_passed: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("reprospan: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CliError> {
    match Cli::parse().command {
        Command::Import { db, bundle } => {
            let source = std::fs::read_to_string(bundle)?;
            let bundle: Bundle = serde_json::from_str(&source)?;
            let mut store = Store::open_and_migrate(db)?;
            store.import_bundle(&bundle)?;
            println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
        Command::Timeline { db, bundle_id } => {
            let store = Store::open_and_migrate(db)?;
            let bundle = store.timeline(&bundle_id)?;
            println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
        Command::Export { db, bundle_id } => {
            let store = Store::open_and_migrate(db)?;
            let bundle = store.export_bundle(&bundle_id)?;
            println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
        Command::Patch { bundle, patch } => {
            let bundle: Bundle = read_json(bundle)?;
            let patch: Patch = read_json(patch)?;
            let patched = bundle.apply_patch(&patch)?;
            println!("{}", serde_json::to_string_pretty(&patched)?);
        }
        Command::Diff { before, after } => {
            let before: Bundle = read_json(before)?;
            let after: Bundle = read_json(after)?;
            let diff = before.semantic_diff(&after)?;
            println!("{}", serde_json::to_string_pretty(&diff)?);
        }
        Command::Eval { bundle, eval } => {
            let bundle: Bundle = read_json(bundle)?;
            let evaluation: Evaluation = read_json(eval)?;
            let result = bundle.evaluate(&evaluation)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.passed {
                return Err(CliError::EvaluationFailed);
            }
        }
        Command::Demo { db } => {
            let bundle: Bundle = serde_json::from_str(BUNDLE_FIXTURE)?;
            let patch: Patch = serde_json::from_str(PATCH_FIXTURE)?;
            let evaluation: Evaluation = serde_json::from_str(EVAL_FIXTURE)?;
            bundle.validate()?;

            let mut store = Store::open_and_migrate(db)?;
            let imported = match store.import_bundle(&bundle) {
                Ok(()) => true,
                Err(StoreError::Conflict(_)) => false,
                Err(error) => return Err(error.into()),
            };
            let recorded = store.export_bundle(&bundle.bundle_id)?;
            let patched = recorded.apply_patch(&patch)?;
            let diff = recorded.semantic_diff(&patched)?;
            let result = patched.evaluate(&evaluation)?;
            let summary = DemoSummary {
                bundle_id: bundle.bundle_id,
                imported,
                exported: true,
                patch_id: patch.patch_id,
                changed_event_count: diff.changed_events.len(),
                eval_id: result.eval_id,
                eval_passed: result.passed,
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
            if !result.passed {
                return Err(CliError::EvaluationFailed);
            }
        }
        Command::Serve { db, listen } => reprospan_server::serve(db, listen).await?,
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T, CliError> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("failed to read input")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON input")]
    Json(#[from] serde_json::Error),
    #[error("invalid replay document: {0}")]
    Core(#[from] reprospan_core::CoreError),
    #[error("local store operation failed: {0}")]
    Store(#[from] StoreError),
    #[error("server failed: {0}")]
    Server(#[from] reprospan_server::ServeError),
    #[error("deterministic evaluation failed")]
    EvaluationFailed,
}
