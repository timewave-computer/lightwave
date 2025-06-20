# SP1-Helios ZK Light-Client operator for Valence
This repository contains a ZK prover that generates and submits SP1-Helios light client proofs
to our coprocessor. The service maintains a chain of proofs that verify the execution state
of the Ethereum network, committing both the execution block height and state root for each
verified block.

# (Re-)Initialization
Most ZK Light Clients (Lodestar, Nimbus, ...) don't maintain the full finality proof history for all slots.
Because of this we have a fallback / re-initialization strategy, that we can use in case our prover goes 
down for an extended period (25 hours +).

In order to leverage the strategy, run the following sequence of commands:

```shell
make build-circuits
```

> [!NOTE]
> It is extremely important to dump the latest elf before generating the circuits.
> If you use the wrong elfs for the circuit generation, then the vks might be invalid
> and proof verification might fail.


To start the service:
```shell
make run
```

To continue an existing service with a valid state db:
```shell
make continue
```

# Technicalities (low-level)

## What is the `Period distance`? 
Every 32 slots an epoch ends. A new period (=sync committee rotation) happens every 256 epochs (=8192) slots.
In order to compute the next ZK Light-Client update we must calculate the period diff from the new head to our
trusted beacon block.

# System Architecture
The system consists of three main components that work together to provide ZK light client proofs:

## Preprocessor
The preprocessor is responsible for preparing the serialized circuit inputs for the Helios program. It:
- Takes a trusted slot as input
- Fetches the latest finalized slot from the consensus layer
- Calculates the period distance between slots
- Gathers necessary updates and finality data
- Serializes all inputs for the Helios program

## Service
The service is the main orchestrator that:
- Calls the Helios prover to generate proofs
- Implements recursive proof verification
- Maintains state of the most recent recursive proof
- In the first round, verifies only the proof at the trusted height
- In subsequent rounds, verifies both the current Helios proof and the previous recursive proof
- Updates its state with each new recursive proof
- Commits execution block height and state root for each verified block

## SP1-Helios
This service depends on [SP1-Helios](https://github.com/succinctlabs/sp1-helios). SP1-Helios is an Ethereum ZK Light Client that can be used
to cryptographically prove the correctness of the consensus protocol. The goal is to obtain trusted roots for a given block so that we can
verify state and events that occurred within that block, without having to trust an external service (relayer).

## Recursion
The recursion crate contains the circuit code that:
- Verifies Helios proof
- Implements the recursive verification logic
- Ensures the chain of proofs is valid and connected
- Maintains the security properties of the light client protocol
- Verifies the execution state root and block height for each block

## Wrapper
The wrapper crate contains the circuit code that:
- Verifies the Recursion proof
- Commits the same public outputs as the Recursion proof

The Wrapper circuit is necessary because we cannot recursively verify the same circuit within itself - this would require deriving the verifying key inside the circuit, which is not possible. Instead, we use a separate wrapper circuit that verifies the recursive proof and maintains the same security properties.

## Verification Logic
In order to be able to generate the first valid proof using our circuit, one has to obtain a valid Helios proof for our trusted checkpoint.
The trusted checkpoint is a pair of Root, Slot that we know is valid and this is the only trust assumption in the protocol.

Moving forward every new proof will be verified against a valid previous proof, e.g. we always have to make a transition from one of the previous
valid Helios checkpoints to a new checkpoint.

For our Valence MVP we expose only the execution state root and block height, because that is all we need to verify stored values in Smart Contracts on Ethereum.
Later we can expose other roots like the beacon header root (default for Helios) or the root of the receipts tree.

## Circuit Inputs and Outputs

### Recursion Circuit
| Input | Description |
|-------|-------------|
| `electra_body_roots` | Merkle roots of the Electra block body components |
| `electra_header` | Electra block header containing slot, proposer index, and roots |
| `helios_proof` | Proof generated by the Helios circuit |
| `helios_public_values` | Public values from the Helios proof |
| `helios_vk` | Verification key for the Helios circuit |
| `previous_wrapper_proof` | Optional previous wrapper proof for recursive verification |
| `previous_wrapper_public_values` | Optional public values from previous wrapper proof |
| `previous_wrapper_vk` | Optional verification key for previous wrapper proof |
| `previous_head` | Slot number of the previous head |

| Output | Description |
|--------|-------------|
| `root` | Execution state root |
| `height` | Execution block height |

### Wrapper Circuit
| Input | Description |
|-------|-------------|
| `recursive_proof` | Proof generated by the Recursion circuit |
| `recursive_public_values` | Public values from the Recursion proof |
| `recursive_vk` | Verification key for the Recursion circuit |

| Output | Description |
|--------|-------------|
| `root` | Execution state root (same as Recursion circuit) |
| `height` | Execution block height (same as Recursion circuit) |
