use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceState {
    pub most_recent_recursive_proof: Option<SP1ProofWithPublicValues>,
    pub most_recent_wrapper_proof: Option<SP1ProofWithPublicValues>,
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
                most_recent_recursive_proof BLOB,
                most_recent_wrapper_proof BLOB,
                trusted_slot INTEGER NOT NULL,
                trusted_height INTEGER NOT NULL,
                trusted_root BLOB NOT NULL,
                update_counter INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn from_env() -> Result<Self> {
        let db_path = std::env::var("SERVICE_STATE_DB_PATH")
            .unwrap_or_else(|_| "service_state.db".to_string());
        let conn = Connection::open(db_path)?;

        // Create the state table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS service_state (
                        id INTEGER PRIMARY KEY CHECK (id = 1),
                        most_recent_recursive_proof BLOB,
                        most_recent_wrapper_proof BLOB,
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
        let recursive_proof_bytes = state
            .most_recent_recursive_proof
            .as_ref()
            .map(|proof| serde_json::to_vec(proof))
            .transpose()?;

        let wrapper_proof_bytes = state
            .most_recent_wrapper_proof
            .as_ref()
            .map(|proof| serde_json::to_vec(proof))
            .transpose()?;

        self.conn.execute(
            "INSERT OR REPLACE INTO service_state (
                id, most_recent_recursive_proof, most_recent_wrapper_proof,
                trusted_slot, trusted_height, trusted_root, update_counter
            ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                recursive_proof_bytes,
                wrapper_proof_bytes,
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
            "SELECT most_recent_recursive_proof,  most_recent_wrapper_proof,
                    trusted_slot, trusted_height, trusted_root, update_counter 
             FROM service_state WHERE id = 1",
        )?;

        let state = stmt
            .query_row([], |row| {
                let recursive_proof_bytes: Option<Vec<u8>> = row.get(0)?;
                let most_recent_recursive_proof = recursive_proof_bytes
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .transpose()
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                let wrapper_proof_bytes: Option<Vec<u8>> = row.get(1)?;
                let most_recent_wrapper_proof = wrapper_proof_bytes
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .transpose()
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(ServiceState {
                    most_recent_recursive_proof,
                    most_recent_wrapper_proof,
                    trusted_slot: row.get(2)?,
                    trusted_height: row.get(3)?,
                    trusted_root: row.get(4)?,
                    update_counter: row.get(5)?,
                })
            })
            .optional()?;

        Ok(state)
    }

    pub fn initialize_state(&self, initial_slot: u64, initial_height: u64) -> Result<ServiceState> {
        let state = ServiceState {
            most_recent_recursive_proof: None,
            most_recent_wrapper_proof: None,
            trusted_slot: initial_slot,
            trusted_height: initial_height,
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
