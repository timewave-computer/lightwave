# Lightwave: Recursive ZK Light Clients in Rust

An extensible, recursive, ZK light client operator service that currently supports both Ethereum (via Helios) and Tendermint chains. This service generates and verifies zero-knowledge proofs of light client state transitions, enabling trustless verification of blockchain state.

## Todos
- Feature gate Light Client specific deps

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

## Getting Started

1. Set the environment variable `CLIENT_BACKEND` to either `"HELIOS"` or `"TENDERMINT"` to choose which light client to use
2. Follow the initialization instructions in the respective documentation:
   - [Helios Initialization](docs/HELIOS.md#re-initialization)
   - [Tendermint Initialization](docs/TENDERMINT.md#re-initialization)
3. Specify the trusted checkpoint for your chosen light client in `crates/service/src/checkpoints.rs`:
   - For Helios: Update `HELIOS_TRUSTED_SLOT` with the desired slot number
   - For Tendermint: Update `TENDERMINT_TRUSTED_HEIGHT` and `TENDERMINT_TRUSTED_ROOT` with the desired height and root hash
4. Start the service using `make run`

## Architecture

The service consists of several key components:

- **Service**: Main orchestrator that manages proof generation and verification
- **Preprocessor**: Prepares inputs for the light client programs
- **Recursion Circuit**: Verifies light client proofs and maintains proof chain
- **Wrapper Circuit**: Verifies recursive proofs and commits outputs

## License

This project is licensed under the MIT License - see the LICENSE file for details. 