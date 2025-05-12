use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceState {
    pub genesis_committee_hash: Option<String>,
    pub most_recent_proof: Option<SP1ProofWithPublicValues>,
    pub trusted_slot: u64,
    pub trusted_height: u64,
    pub trusted_root: [u8; 32],
    pub update_counter: u64,
}

pub struct StateManager {
    conn: Connection,
}

impl StateManager {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create the state table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS service_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                genesis_committee_hash TEXT,
                most_recent_proof BLOB,
                trusted_slot INTEGER NOT NULL,
                trusted_height INTEGER NOT NULL,
                trusted_root BLOB NOT NULL,
                update_counter INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn save_state(&self, state: &ServiceState) -> Result<()> {
        let proof_bytes = state
            .most_recent_proof
            .as_ref()
            .map(|proof| serde_json::to_vec(proof))
            .transpose()?;

        self.conn.execute(
            "INSERT OR REPLACE INTO service_state (
                id, genesis_committee_hash, most_recent_proof, 
                trusted_slot, trusted_height, trusted_root, update_counter
            ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                state.genesis_committee_hash,
                proof_bytes,
                state.trusted_slot,
                state.trusted_height,
                state.trusted_root,
                state.update_counter,
            ],
        )?;

        Ok(())
    }

    pub fn load_state(&self) -> Result<Option<ServiceState>> {
        let mut stmt = self.conn.prepare(
            "SELECT genesis_committee_hash, most_recent_proof, 
                    trusted_slot, trusted_height, trusted_root, update_counter 
             FROM service_state WHERE id = 1",
        )?;

        let state = stmt
            .query_row([], |row| {
                let proof_bytes: Option<Vec<u8>> = row.get(1)?;
                let most_recent_proof = proof_bytes
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .transpose()
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(ServiceState {
                    genesis_committee_hash: row.get(0)?,
                    most_recent_proof,
                    trusted_slot: row.get(2)?,
                    trusted_height: row.get(3)?,
                    trusted_root: row.get(4)?,
                    update_counter: row.get(5)?,
                })
            })
            .optional()?;

        Ok(state)
    }

    pub fn initialize_state(&self, initial_slot: u64) -> Result<ServiceState> {
        let state = ServiceState {
            genesis_committee_hash: None,
            most_recent_proof: None,
            trusted_slot: initial_slot,
            trusted_height: 0,
            trusted_root: [0; 32],
            update_counter: 0,
        };

        self.save_state(&state)?;
        Ok(state)
    }

    /// Deletes the entire state file.
    /// Note: This will close the current connection and delete the database file.
    /// The StateManager instance will be consumed by this operation.
    pub fn delete_state(self) -> Result<()> {
        // Clone the path before dropping the connection
        let db_path = self
            .conn
            .path()
            .ok_or_else(|| anyhow::anyhow!("Could not get database path"))?
            .to_path_buf(); // <-- clone the Path

        // Now we can safely drop the connection
        drop(self.conn);

        // Then delete the file
        std::fs::remove_file(db_path)?;
        Ok(())
    }
}
