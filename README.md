# Lightwave: Recursive ZK Light Clients in Rust

An extensible, recursive, ZK light client operator service that currently supports both Ethereum (via Helios) and Tendermint chains. This service generates and verifies zero-knowledge proofs of light client state transitions, enabling trustless verification of blockchain state.

## Documentation

- [Helios Documentation](docs/integrations/HELIOS.md) - Documentation for the Ethereum light client implementation
- [Tendermint Documentation](docs/integrations/TENDERMINT.md) - Documentation for the Tendermint light client implementation

## Features

- Support for both Ethereum and Tendermint light clients
- Recursive proof verification
- State persistence
- REST API for proof retrieval
- Configurable trusted checkpoints
- Automatic proof generation and verification

## Makefile Commands

The project provides several Makefile commands for different use cases:

### `make build-circuits`
**One-time setup command** - Generates the ZK circuits and ELF files needed for proof generation and verification. This command:
- Deletes existing state
- Generates recursive circuits for both Helios and Tendermint
- Dumps ELF files to `elfs/variable/`
- Generates wrapper circuits
- Dumps final ELF files

> [!NOTE] 
> This command should only be run once per deployment. Running it on different machines or
> after circuit code modifications will produce new ELF files that won't match previously 
> generated ones, potentially leading to invalid proofs.

### `make run`
**Fresh start from hardcoded checkpoint** - Starts the service with a clean slate. This command:
- **Deletes the database** and all existing state
- Starts proving from the first checkpoint all the way to the current head
- Initializes new state based on the trusted checkpoints in `crates/service/src/checkpoints.rs`

> [!WARNING]
> This command will **prune the database** and restart proof generation from the beginning. 
> Use this only when you want to completely reset the service state.

### `make continue`
**Resume prover** - Continues the service from where it left off. This command:
- Loads existing state from the database
- Continues proof generation from the last processed checkpoint
- Preserves all previously generated proofs and state

> 💡 **Recommended**: Use this command for normal operation and after service restarts.

## Getting Started

1. Set the environment variable `CLIENT_BACKEND` to either `"HELIOS"` or `"TENDERMINT"` to choose which light client to use
2. Follow the initialization instructions in the respective documentation:
   - [Helios Initialization](docs/integrations/HELIOS.md#re-initialization)
   - [Tendermint Initialization](docs/integrations/TENDERMINT.md#re-initialization)
3. Specify the trusted checkpoint for your chosen light client in `crates/service/src/checkpoints.rs`:
   - For Helios: Update `HELIOS_TRUSTED_SLOT` with the desired slot number
   - For Tendermint: Update `TENDERMINT_TRUSTED_HEIGHT` and `TENDERMINT_TRUSTED_ROOT` with the desired height and root hash
4. **First time setup**: Run `make build-circuits` to generate the required circuits and ELF files
5. **Start the service**: 
   - For fresh start: `make run` (⚠️ **prunes database**)
   - For normal operation: `make continue`

## Architecture

The service consists of several key components:

- **Service**: Main orchestrator that manages proof generation and verification
- **Preprocessor**: Prepares inputs for the light client programs
- **Recursion Circuit**: Verifies light client proofs and maintains proof chain
- **Wrapper Circuit**: Verifies recursive proofs and commits outputs
