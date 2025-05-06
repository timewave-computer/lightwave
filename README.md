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
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0x4fa350fa61823a1b1b752c325220b1fabcf2a9c27da48032614c756a3b486c53, prevHead: 7560896, prevSyncCommitteeHash: 0x04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f, newHead: 7568640, newHeader: 0xfbdf0a5e20f5e9359436cffae146423f93ccc089a33138b327db2ce66919b1ea, executionStateRoot: 0xa6771daabc69d0dc2530e18ec0dabfcfeb32db957c427ff5ca7e441dbe05aee9, syncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, nextSyncCommitteeHash: 0x0000000000000000000000000000000000000000000000000000000000000000 }"), trusted_slot: 7568640 } 

[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
Period distance is 0, defaulting to 1
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0xfbdf0a5e20f5e9359436cffae146423f93ccc089a33138b327db2ce66919b1ea, prevHead: 7568640, prevSyncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, newHead: 7568672, newHeader: 0x8824a92b7bcf6c5af21b21ec585530ae855e8b8bd2064b2e0f549ce6e6808ee2, executionStateRoot: 0xa53da213c622710d058fcf5ab870983fe36245abf202c284d7b6b72b14a9b156, syncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, nextSyncCommitteeHash: 0xb784d3f228e9a40c08e991ae0b924c8350e098f7526cab2e76f52f78c16e3f9e }"), trusted_slot: 7568672 } 

[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
[Warning]: Waiting for new slot to be finalized, retry in 60 seconds!
Period distance is 0, defaulting to 1
New Service State: ServiceState { genesis_committee_hash: Some("04dea69eb81ecfc6d4c9268d97df4b64cff46decd0eabd005a132b720189117f"), most_recent_proof_outputs: Some("ProofOutputs { prevHeader: 0x8824a92b7bcf6c5af21b21ec585530ae855e8b8bd2064b2e0f549ce6e6808ee2, prevHead: 7568672, prevSyncCommitteeHash: 0x5ec9766f1473b7f36119d46895ca82e8e7c3752aaf84414439b3a3e3fc5bf9c0, newHead: 7568704, newHeader: 0x5619fa8cba1be46ca780c001a5493b1039426a2a20756b8c6c149642f61289d6, executionStateRoot: 0xd16d76a1724cc39c25a016de4ed40335efebfea23526d3f2cfba76
```


# Technicalities (low-level)

## What is the `Period distance`? 
Every 32 slots an epoch ends. A new period (=sync committee rotation) happens every 256 epochs (=8192) slots.
In order to compute the next ZK Light-Client update we must calculate the period diff from the new head to our
trusted beacon block.
