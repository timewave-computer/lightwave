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

use crate::checkpoints::{HELIOS_TRUSTED_SLOT, TENDERMINT_TRUSTED_HEIGHT, TENDERMINT_TRUSTED_ROOT};
pub mod checkpoints;

/// Command line arguments for the service
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Delete the state file before starting
    #[arg(long)]
    delete: bool,

    /// Initial slot number to start from (only used when initializing new state)
    #[arg(long)]
    generate_recursion_circuit: bool,

    /// Generate the wrapper circuit
    #[arg(long)]
    generate_wrapper_circuit: bool,

    /// Dump the ELFs as bytes
    #[arg(long)]
    dump_elfs: bool,
}

// Binary artifacts for the various circuits used in the light client
pub const HELIOS_ELF: &[u8] = include_bytes!("../../../elfs/constant/sp1-helios-elf");
pub const TENDERMINT_ELF: &[u8] = include_bytes!("../../../elfs/constant/sp1-tendermint-elf");
pub const RECURSIVE_ELF_HELIOS: &[u8] = include_elf!("helios-recursion-circuit");
pub const WRAPPER_ELF_HELIOS: &[u8] = include_elf!("helios-wrapper-circuit");
pub const RECURSIVE_ELF_TENDERMINT: &[u8] = include_elf!("tendermint-recursion-circuit");
pub const WRAPPER_ELF_TENDERMINT: &[u8] = include_elf!("tendermint-wrapper-circuit");

/// Main entry point for the light client service.
///
/// This function:
/// 1. Initializes the service state with a trusted slot
/// 2. Sets up the prover client and circuit artifacts
/// 3. Enters a loop that:
///    - Generates proofs for new blocks (Helios or Tendermint depending on mode)
///    - Verifies proofs recursively
///    - Updates the service state with new trusted information
///    - Commits execution block height and state root instead of beacon header
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level and clean formatting
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Parse command line arguments
    let args = Args::parse();
    let client = ProverClient::from_env();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get server port from environment or use default
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "7778".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Create router for API endpoints
    let app = Router::new().route("/", get(get_proof));

    // Create a shutdown signal handler for graceful shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let app = app.into_make_service();

    // Get consensus URL from environment
    let consensus_url = std::env::var("SOURCE_CONSENSUS_RPC_URL").unwrap_or_default();

    // Get database path from environment or use default
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
        tracing::info!("State file deleted successfully");
        return Ok(());
    }

    // Get client backend mode from environment
    let mode = std::env::var("CLIENT_BACKEND").unwrap_or_else(|_| "TENDERMINT".to_string());

    // Set up ELF file paths
    let elfs_path = std::env::var("ELFS_OUT").unwrap_or_else(|_| "elfs/variable".to_string());
    let helios_recursive_elf_path = Path::new(&elfs_path).join("helios-recursive-elf.bin");
    let helios_wrapper_elf_path = Path::new(&elfs_path).join("helios-wrapper-elf.bin");
    let tendermint_recursive_elf_path = Path::new(&elfs_path).join("tendermint-recursive-elf.bin");
    let tendermint_wrapper_elf_path = Path::new(&elfs_path).join("tendermint-wrapper-elf.bin");

    // Generate the Recursion Circuit if requested
    if args.generate_recursion_circuit {
        // Initialize the preprocessor with the current trusted slot
        let preprocessor = Preprocessor::new(HELIOS_TRUSTED_SLOT);
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
        let template = include_str!("../../integrations/sp1-helios/circuit/src/blueprint.rs");

        // Generate the Helios recursive circuit
        let (_, helios_vk) = client.setup(HELIOS_ELF);
        let generated_code = template
            .replace("{ committee_hash }", &committee_hash_formatted)
            .replace("{ trusted_head }", &HELIOS_TRUSTED_SLOT.to_string())
            .replace("{ helios_vk }", &helios_vk.bytes32());
        write(
            "crates/integrations/sp1-helios/circuit/src/main.rs",
            generated_code,
        )
        .context("Failed to generate recursive circuit from blueprint")?;

        // Generate the Tendermint recursive circuit
        let template = include_str!("../../integrations/sp1-tendermint/circuit/src/blueprint.rs");
        let (_, tendermint_vk) = client.setup(TENDERMINT_ELF);
        let generated_code = template
            .replace("{ trusted_height }", &TENDERMINT_TRUSTED_HEIGHT.to_string())
            .replace(
                "{ trusted_root }",
                &format!("{:?}", TENDERMINT_TRUSTED_ROOT),
            )
            .replace("{ tendermint_vk }", &tendermint_vk.bytes32());
        write(
            "crates/integrations/sp1-tendermint/circuit/src/main.rs",
            generated_code,
        )
        .context("Failed to generate recursive circuit from blueprint")?;

        tracing::info!("Recursive circuit generated successfully");
        return Ok(());
    }

    // Generate the Wrapper Circuit if requested
    if args.generate_wrapper_circuit {
        let client = ProverClient::from_env();
        let (_, helios_vk) = client.setup(RECURSIVE_ELF_HELIOS);
        let helios_vk_bytes = helios_vk.bytes32();

        let (_, tendermint_vk) = client.setup(RECURSIVE_ELF_TENDERMINT);
        let tendermint_vk_bytes = tendermint_vk.bytes32();

        let template =
            include_str!("../../integrations/sp1-helios/wrapper-circuit/src/blueprint.rs");
        let generated_code =
            template.replace("{ recursive_vk }", &format!("{:?}", helios_vk_bytes));

        // Generate the Helios wrapper circuit
        write(
            "crates/integrations/sp1-helios/wrapper-circuit/src/main.rs",
            generated_code,
        )
        .context("Failed to generate wrapper circuit from blueprint")?;

        let template =
            include_str!("../../integrations/sp1-tendermint/wrapper-circuit/src/blueprint.rs");

        // Generate the Tendermint wrapper circuit
        let generated_code =
            template.replace("{ recursive_vk }", &format!("{:?}", tendermint_vk_bytes));
        write(
            "crates/integrations/sp1-tendermint/wrapper-circuit/src/main.rs",
            generated_code,
        )
        .context("Failed to generate wrapper circuit from blueprint")?;

        tracing::info!("Wrapper circuit generated successfully");
        return Ok(());
    }

    // Dump the ELFs as bytes if requested
    if args.dump_elfs {
        std::fs::create_dir_all(&elfs_path)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&elfs_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create ELF directory")?;
        }

        // Write Helios ELFs
        std::fs::write(&helios_recursive_elf_path, RECURSIVE_ELF_HELIOS).context(format!(
            "Failed to dump recursive ELF to {}",
            helios_recursive_elf_path.display()
        ))?;
        std::fs::write(&helios_wrapper_elf_path, WRAPPER_ELF_HELIOS).context(format!(
            "Failed to dump wrapper ELF to {}",
            helios_wrapper_elf_path.display()
        ))?;

        // Write Tendermint ELFs
        std::fs::write(&tendermint_recursive_elf_path, RECURSIVE_ELF_TENDERMINT).context(
            format!(
                "Failed to dump recursive ELF to {}",
                tendermint_recursive_elf_path.display()
            ),
        )?;
        std::fs::write(&tendermint_wrapper_elf_path, WRAPPER_ELF_TENDERMINT).context(format!(
            "Failed to dump wrapper ELF to {}",
            tendermint_wrapper_elf_path.display()
        ))?;

        tracing::info!("ELFs dumped successfully");
        return Ok(());
    }

    // Load or initialize the service state
    let state_manager = StateManager::new(Path::new(&db_path))?;
    let service_state = match state_manager.load_state()? {
        Some(state) => state,
        None => match mode.as_str() {
            "TENDERMINT" => state_manager
                .initialize_state(TENDERMINT_TRUSTED_HEIGHT, TENDERMINT_TRUSTED_HEIGHT)?,
            "HELIOS" => state_manager.initialize_state(HELIOS_TRUSTED_SLOT, 0)?,
            _ => state_manager.initialize_state(HELIOS_TRUSTED_SLOT, 0)?,
        },
    };

    // Start the API server in a separate task
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

    // Handle shutdown signals (Ctrl+C)
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for ctrl+c: {}", e);
        }
        info!("Received shutdown signal");
        let _ = shutdown_tx.send(());
    });

    // Verify that required ELF files exist
    if !helios_recursive_elf_path.exists() {
        println!(
            "Recursive ELF not found at {}, please run with --dump-elfs",
            helios_recursive_elf_path.display()
        );
        return Err(anyhow::anyhow!("Recursive ELF not found"));
    }

    // Load the appropriate ELF files based on the selected mode
    let (recursive_elf, wrapper_elf) = match mode.as_str() {
        "TENDERMINT" => {
            // Read bytes of recursive-elf and wrapper-elf for Tendermint
            let recursive_elf = std::fs::read(&tendermint_recursive_elf_path).context(format!(
                "Failed to read recursive elf from {}",
                tendermint_recursive_elf_path.display()
            ))?;

            let wrapper_elf = std::fs::read(&tendermint_wrapper_elf_path).context(format!(
                "Failed to read wrapper elf from {}",
                tendermint_wrapper_elf_path.display()
            ))?;

            (recursive_elf, wrapper_elf)
        }
        "HELIOS" => {
            // Read bytes of recursive-elf and wrapper-elf for Helios
            let recursive_elf = std::fs::read(&helios_recursive_elf_path).context(format!(
                "Failed to read recursive elf from {}",
                helios_recursive_elf_path.display()
            ))?;

            let wrapper_elf = std::fs::read(&helios_wrapper_elf_path).context(format!(
                "Failed to read wrapper elf from {}",
                helios_wrapper_elf_path.display()
            ))?;

            (recursive_elf, wrapper_elf)
        }
        _ => {
            panic!("Invalid mode: {:?}", mode);
        }
    };

    // Start the prover service loop in a separate task
    let service_handle = tokio::spawn(run_prover_loop(
        state_manager,
        service_state,
        recursive_elf,
        wrapper_elf,
        consensus_url,
    ));

    // Wait for both tasks to conclude
    let (server_result, service_result) = tokio::join!(server_handle, service_handle);

    // Handle any errors from the tasks
    if let Err(e) = server_result {
        error!("API server crashed: {}", e);
        return Err(anyhow::anyhow!("{}", e));
    }

    if let Err(e) = service_result {
        error!("Prover service crashed: {}", e);
        return Err(anyhow::anyhow!("{}", e));
    }

    Ok(())
}
