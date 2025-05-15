// This is the main service that orchestrates the light client update process.
// It manages the state of the light client, generates and verifies proofs,
// and maintains a chain of trusted state transitions.

use anyhow::{Context, Result};
use axum::{Router, routing::get};
use std::{fs::write, path::Path};
mod api;
use api::get_proof;
use clap::Parser;
use preprocessor::Preprocessor;
use sp1_helios_primitives::types::ProofInputs as HeliosInputs;
use sp1_sdk::{HashableKey, ProverClient, include_elf};
use tokio::signal;
use tracing::{error, info};
mod preprocessor;
mod state;
use state::StateManager;
use tree_hash::TreeHash;
mod prover;
use prover::run_prover_loop;

/// Command line arguments for the service
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Delete the state file before starting
    #[arg(long)]
    delete: bool,

    /// Initial slot number to start from (only used when initializing new state)
    #[arg(long)]
    generate_recursion_circuit: Option<u64>,

    /// Generate the wrapper circuit
    #[arg(long)]
    generate_wrapper_circuit: bool,

    /// dump the elfs as bytes
    #[arg(long)]
    dump_elfs: bool,
}

// Binary artifacts for the various circuits used in the light client
pub const HELIOS_ELF: &[u8] = include_bytes!("../../elfs/constant/sp1-helios-elf");
pub const RECURSIVE_ELF_RUNTIME: &[u8] = include_elf!("recursion-circuit");
pub const WRAPPER_ELF_RUNTIME: &[u8] = include_elf!("wrapper-circuit");
pub const DEFAULT_SLOT: u64 = 11709792;

/// Main entry point for the light client service.
///
/// This function:
/// 1. Initializes the service state with a trusted slot
/// 2. Sets up the prover client and circuit artifacts
/// 3. Enters a loop that:
///    - Generates Helios proofs for new blocks
///    - Verifies proofs recursively
///    - Updates the service state with new trusted information
///    - Commits execution block height and state root instead of beacon header
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get server port from environment or use default
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Create router
    let app = Router::new().route("/", get(get_proof));

    // Create a shutdown signal handler
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let app = app.into_make_service();

    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("API server listening on {}", addr);
                listener
            }
            Err(e) => {
                error!("Failed to bind to {}: {}", addr, e);
                return Err(e);
            }
        };

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
                info!("API server shutting down gracefully");
            })
            .await
            .map_err(|e| {
                error!("API server error: {}", e);
                e
            })
    });

    // Handle shutdown signals
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for ctrl+c: {}", e);
        }
        info!("Received shutdown signal");
        let _ = shutdown_tx.send(());
    });

    // Load environment variables and initialize the prover client
    dotenvy::dotenv().ok();

    let consensus_url = std::env::var("SOURCE_CONSENSUS_RPC_URL").unwrap_or_default();

    let db_path =
        std::env::var("SERVICE_STATE_DB_PATH").unwrap_or_else(|_| "service_state.db".to_string());

    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).context("Failed to create database directory")?;
    }

    // Initialize the state manager with a database file
    let state_manager = StateManager::new(Path::new(&db_path))?;
    // Delete state if --delete flag is set
    if args.delete {
        state_manager.delete_state()?;
        println!("State file deleted successfully");
        return Ok(());
    }

    let state_manager = StateManager::new(Path::new(&db_path))?; // Load or initialize the service state
    let service_state = match state_manager.load_state()? {
        Some(state) => state,
        None => state_manager.initialize_state(DEFAULT_SLOT)?,
    };

    let elfs_path = std::env::var("ELFS_OUT").unwrap_or_else(|_| "elfs/variable".to_string());
    let recursive_elf_path = Path::new(&elfs_path).join("recursive-elf.bin");
    let wrapper_elf_path = Path::new(&elfs_path).join("wrapper-elf.bin");

    // Generate the Recursion Circuit
    if args.generate_recursion_circuit.is_some() {
        let initial_slot = args.generate_recursion_circuit.unwrap_or(DEFAULT_SLOT);
        // Initialize the preprocessor with the current trusted slot
        let preprocessor =
            Preprocessor::new(args.generate_recursion_circuit.expect("Missing Slot"));
        // Get the next block's inputs for proof generation
        let inputs = preprocessor.run().await?;

        let helios_inputs: HeliosInputs = serde_cbor::from_slice(&inputs)?;
        let trusted_committee_hash = helios_inputs
            .store
            .current_sync_committee
            .clone()
            .tree_hash_root()
            .to_vec();

        let committee_hash_formatted = format!("{:?}", trusted_committee_hash);
        let template = include_str!("../../recursion/circuit/src/blueprint.rs");

        let generated_code = template
            .replace("{ committee_hash }", &committee_hash_formatted)
            .replace("{ trusted_head }", &initial_slot.to_string());
        write("recursion/circuit/src/main.rs", generated_code)
            .context("Failed to generate recursive circuit from blueprint")?;

        println!("Recursive circuit generated successfully");
        return Ok(());
    }

    // Generate the Wrapper Circuit
    if args.generate_wrapper_circuit {
        let client = ProverClient::new();
        let (_, vk) = client.setup(RECURSIVE_ELF_RUNTIME);
        let vk_bytes = vk.bytes32();

        let template = include_str!("../../recursion/wrapper-circuit/src/blueprint.rs");
        let generated_code = template.replace("{ recursive_vk }", &format!("\"{}\"", vk_bytes));

        write("recursion/wrapper-circuit/src/main.rs", generated_code)
            .context("Failed to generate wrapper circuit from blueprint")?;

        println!("Wrapper circuit generated successfully");
        return Ok(());
    }

    // Dump the ELFs as bytes
    if args.dump_elfs {
        std::fs::create_dir_all(&elfs_path)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&elfs_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create ELF directory")?;
        }

        std::fs::write(&recursive_elf_path, RECURSIVE_ELF_RUNTIME).context(format!(
            "Failed to dump recursive ELF to {}",
            recursive_elf_path.display()
        ))?;
        std::fs::write(&wrapper_elf_path, WRAPPER_ELF_RUNTIME).context(format!(
            "Failed to dump wrapper ELF to {}",
            wrapper_elf_path.display()
        ))?;

        println!("ELFs dumped successfully");
        return Ok(());
    }

    if !recursive_elf_path.exists() {
        println!(
            "Recursive ELF not found at {}, please run with --dump-elfs",
            recursive_elf_path.display()
        );
        return Err(anyhow::anyhow!("Recursive ELF not found"));
    }

    // read bytes of recursive-elf and wrapper-elf
    let recursive_elf = std::fs::read(&recursive_elf_path).context(format!(
        "Failed to read recursive elf from {}",
        recursive_elf_path.display()
    ))?;

    let wrapper_elf = std::fs::read(&wrapper_elf_path).context(format!(
        "Failed to read wrapper elf from {}",
        wrapper_elf_path.display()
    ))?;

    // Start the service loop in a separate task
    let service_handle = tokio::spawn(run_prover_loop(
        state_manager,
        service_state,
        recursive_elf,
        wrapper_elf,
        consensus_url,
    ));

    // Wait for both tasks to conclude
    tokio::select! {
        server_result = server_handle => {
            if let Err(e) = server_result {
                error!("Server task failed: {}", e);
                return Err(anyhow::anyhow!("Server task failed: {}", e));
            }
        }
        service_result = service_handle => {
            if let Err(e) = service_result {
                error!("Service loop failed: {}", e);
                return Err(anyhow::anyhow!("Service loop failed: {}", e));
            }
        }
    }

    Ok(())
}
