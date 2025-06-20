# Lightwave Code Review 
This document highlights the key pieces of source code
that need to be reviewed and maintained in order for lightwave to be 
production-ready.

## Key Components
1. The `recursive and wrapper circuits` for integrated ZK light clients
are located in `crates/integrations/PROJECT`. 

The Helios circuit
has not changed recently and has been tested extensively.

The Tendermint circuit is less complex due to no committee checks and 
the absence of beacon chain headers.

2. The `main prover loop` that orchestrates the proof generation for the selected
ZK light client is located in `crates/service/src/prover.rs`.

Critical errors are propagated back to the task that is spawned and joined together with the
simple API in `crates/service/src/main.rs`. 

Less critical errors - like thos occuring in the underlying ZK light clients, are logged but 
do not terminate runtime.

3. The circuit inputs for the Ethereum ZK light client are created by the Preprocessor 
located in `crates/service/preprocessor/*`.

The Preprocessor prepares data such as the signed light client updates in range and
the most recent finalized header. The `get_updates` function is mostly copied from 
`SP1-Helios`, but we are using exact period range instead of a MAX value, because 
the Lodestar API endpoint will return an error if we request an update for a value
that is out of bounds. This is tested and ran without issues for 2+ weeks.

### Automated circuit generation from checkpoints.rs
Currently all circuits are automatically generated when running `make build-circuits`, filling in the constants specified in `crates/service/src/checkpoints.rs` into the `blueprint.rs` of each circuit (see for example `crates/integrations/sp1-helios/circuit/src/blueprint.rs`) for the recursive circuit of the Helios prover, or `crates/integrations/sp1-helios/wrapper-circuit/src/blueprint.rs` for the wrapper circuit of the Helios prover.

`make build-circuits` should only be run once and then the ELF (VK, PK) should be distributed.
If `make build-circuits` is run on another machine, or after the circuit code has been modified, then it will produce a new ELF file that will not match the previously generated one -> this can lead to the generation of invalid proofs using the new PK against the initial VK.

> [!WARNING]
> An ELF contains a pair of proving key, verifying key that
> is fully deterministic and unique for the circuit and machine
> that it was compiled on.

## Work in progress (post release 1.0.0)

### Feature gateing dependencies
Currently we pull in all deps for Helios and Tendermint no matter which light client is selected. Feature-gateing the dependencies is work in progress and will be part of the next
tagged release. The reason why this isn't done yet is because it requires continuous testing
to ensure no mistakes are made.

The codebase currently is fully functional for Tendermint and Helios, it's just a matter of
improving the dependency resolution.

### Allow users to specify hex values of trusted root
Currently `checkpoints.rs` defines the bytes as [u8;32], from the next release tag on this
will be the hex encoded string.


## Necessary Workarounds
1. Clear docker containers manually in prover loop
SP1 made the mistake to register a global ctrlc handler in the GPU prover task.
The reason why they did this is because they wanted the docker container to be terminated on ctrl c, however only one ctrlc handler can be registered at a time.

Therefore, since the handler is not registered globally but instead inside a child process, we have to manually stop the docker image before generating a new proof. To be extra sure we currently prune the affected image when the prover loop enters and before spawning the prover sub-tasks for generating the unwrapped ZK light client proofs.

I am trying to convince them to remove this ctrlc handler from the cuda prover, but it looks like they want to keep it for the reason above.
