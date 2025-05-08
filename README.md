# SP1-Helios ZK Light-Client operator for Valence
This repository contains a ZK prover that generates and submits SP1-Helios light client proofs
to our coprocessor.

To start the service:

```shell
cargo run --bin service --release -- --nocapture
```

Example output:

```
Period distance: 1

[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 23.708µs
Reading R1CS took 6.98379225s
Reading proving key took 554.491458ms
Reading witness file took 134.917µs
Deserializing JSON data took 4.292958ms
Generating witness took 127.62825ms
23:49:56 DBG constraint system solver done nbConstraints=8173052 took=1204.108916
23:50:03 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6542.392042
Generating proof took 7.746944459s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
23:50:03 DBG verifier done backend=groth16 curve=bn254 took=0.942083
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0xb048fc6b159cf756a65e5356b412c1047aa69dc998f3a690c7a5ff23f27033ad, prevHead: 7553088, prevSyncCommitteeHash: 0x04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f, newHead: 7568960, newHeader: 0xd9a0fc7b241739b725af50f5c4ef161df8d96e343b9d5a64e625e55a236f38df, executionStateRoot: 0x2b6192b32eb25481b4fda777b800a860304d32364fc51b9aeff748622e5ce734, syncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, nextSyncCommitteeHash: 0x0000000000000000000000000000000000000000000000000000000000000000 }"), trusted_slot: 7568960 } 

Alive for: 6075.875394875s
Period distance: 1
[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 14.75µs
Reading witness file took 185.042µs
Deserializing JSON data took 4.583167ms
Generating witness took 125.128ms
01:47:29 DBG constraint system solver done nbConstraints=8173052 took=1293.903875
01:47:36 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6753.252292
Generating proof took 8.047312334s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
01:47:36 DBG verifier done backend=groth16 curve=bn254 took=0.878208
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0xd9a0fc7b241739b725af50f5c4ef161df8d96e343b9d5a64e625e55a236f38df, prevHead: 7568960, prevSyncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, newHead: 7569472, newHeader: 0x45e41ee39cfdcb07e21093fc0b8d0d264d68951c2291bca8e74a5320070036c9, executionStateRoot: 0x1a81ed528d49f26ced035eb6c1a666b21dd4c82bbbdc72a069dee4db2d6a7d89, syncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, nextSyncCommitteeHash: 0x0000000000000000000000000000000000000000000000000000000000000000 }"), trusted_slot: 7569472 } 

Alive for: 13129.475490875s
Period distance: 0
Period distance is 0, defaulting to 1
[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 13.166µs
Reading witness file took 75.583µs
Deserializing JSON data took 4.275333ms
Generating witness took 125.587625ms
03:36:05 DBG constraint system solver done nbConstraints=8173052 took=1165.303375
03:36:11 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6510.602625
Generating proof took 7.676023041s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
03:36:11 DBG verifier done backend=groth16 curve=bn254 took=0.876167
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0x45e41ee39cfdcb07e21093fc0b8d0d264d68951c2291bca8e74a5320070036c9, prevHead: 7569472, prevSyncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, newHead: 7570048, newHeader: 0x7c086b042613974c3996868e28d425968c2ae55cd14d8dc8aed0be34192f87ba, executionStateRoot: 0x8038fd9b076fdb0cfc12140308fa71f877a11ee80f08d5b5f2904cdc5ab4127f, syncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, nextSyncCommitteeHash: 0x5ced1510ae6bc035293322a5958000c3e98b204aa180346f4115b2e36901e1e9 }"), trusted_slot: 7570048 } 

Alive for: 19644.54786975s
Period distance: 0
Period distance is 0, defaulting to 1
[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 9.083µs
Reading witness file took 116.125µs
Deserializing JSON data took 4.163208ms
Generating witness took 124.97025ms
05:48:20 DBG constraint system solver done nbConstraints=8173052 took=1153.970042
05:48:27 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6484.995667
Generating proof took 7.639079834s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
05:48:27 DBG verifier done backend=groth16 curve=bn254 took=0.876875
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0x7c086b042613974c3996868e28d425968c2ae55cd14d8dc8aed0be34192f87ba, prevHead: 7570048, prevSyncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, newHead: 7570592, newHeader: 0xcbb25c7f87f5bef3917df98c2f03697e2839960789022c81245c48754af9325e, executionStateRoot: 0x75ba2ada30456930cd40ef8b83c11817d757017adcf09f0f3d80d45bf16a7aab, syncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, nextSyncCommitteeHash: 0x5ced1510ae6bc035293322a5958000c3e98b204aa180346f4115b2e36901e1e9 }"), trusted_slot: 7570592 } 

Alive for: 27580.139678083s
Period distance: 0
Period distance is 0, defaulting to 1
[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 8.333µs
Reading witness file took 77.416µs
Deserializing JSON data took 4.244458ms
Generating witness took 125.349708ms
07:37:34 DBG constraint system solver done nbConstraints=8173052 took=1150.690167
07:37:40 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6524.74525
Generating proof took 7.67555275s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
07:37:40 DBG verifier done backend=groth16 curve=bn254 took=0.885542
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0xcbb25c7f87f5bef3917df98c2f03697e2839960789022c81245c48754af9325e, prevHead: 7570592, prevSyncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, newHead: 7571264, newHeader: 0x30bb515f5438226f3e6abe65f3ae996d71edae7313b42bddf99dd68bed26d2eb, executionStateRoot: 0x66844011c14cd5b96706eccfe86a94854aa9c674f1f29b7305e2b31407c277ed, syncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, nextSyncCommitteeHash: 0x5ced1510ae6bc035293322a5958000c3e98b204aa180346f4115b2e36901e1e9 }"), trusted_slot: 7571264 } 

Alive for: 34133.186072083s
Period distance: 0
Period distance is 0, defaulting to 1
[sp1] groth16 circuit artifacts already seem to exist at /Users/chef/.sp1/circuits/groth16/v4.0.0-rc.3. if you want to re-download them, delete the directory
Setting environment variables took 8µs
Reading witness file took 74.792µs
Deserializing JSON data took 4.239625ms
Generating witness took 125.2015ms
09:38:32 DBG constraint system solver done nbConstraints=8173052 took=1180.372416
09:38:39 DBG prover done acceleration=none backend=groth16 curve=bn254 nbConstraints=8173052 took=6499.226542
Generating proof took 7.679710708s
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
ignoring uninitialized slice: Vars []frontend.Variable
09:38:39 DBG verifier done backend=groth16 curve=bn254 took=0.882792
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0x30bb515f5438226f3e6abe65f3ae996d71edae7313b42bddf99dd68bed26d2eb, prevHead: 7571264, prevSyncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, newHead: 7571808, newHeader: 0x813a78cf7a544eaebc32d4e74eca2f95ccc7c6c87b8e4bd70f46ad9cf9d7d4e2, executionStateRoot: 0xa2befcaa4436b8b3cb11bebbffa02427552b87122502c401b72090a74f948f5f, syncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e, nextSyncCommitteeHash: 0x5ced1510ae6bc035293322a5958000c3e98b204aa180346f4115b2e36901e1e9 }"), trusted_slot: 7571808 } 

Alive for: 41391.867946583s
Period distance: 0
Period distance is 0, defaulting to 1

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

## Recursion
The recursion component contains the circuit code that:
- Verifies Helios proofs
- Implements the recursive verification logic
- Ensures the chain of proofs is valid and connected
- Maintains the security properties of the light client protocol