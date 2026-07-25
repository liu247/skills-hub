use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::{Context, Result};

const SERVICE_NAME: &str = "skills-hub.mcp";

pub trait CredentialStore: Send + Sync {
    fn set(&self, server_id: &str, env_var: &str, value: &str) -> Result<()>;
    fn get(&self, server_id: &str, env_var: &str) -> Result<Option<String>>;
    fn delete(&self, server_id: &str, env_var: &str) -> Result<()>;
}

#[derive(Default)]
pub struct MemoryCredentialStore {
    entries: Mutex<BTreeMap<(String, String), String>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn set(&self, server_id: &str, env_var: &str, value: &str) -> Result<()> {
        self.entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential store lock poisoned"))?
            .insert(
                (server_id.to_string(), env_var.to_string()),
                value.to_string(),
            );
        Ok(())
    }

    fn get(&self, server_id: &str, env_var: &str) -> Result<Option<String>> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential store lock poisoned"))?
            .get(&(server_id.to_string(), env_var.to_string()))
            .cloned())
    }

    fn delete(&self, server_id: &str, env_var: &str) -> Result<()> {
        self.entries
            .lock()
            .map_err(|_| anyhow::anyhow!("credential store lock poisoned"))?
            .remove(&(server_id.to_string(), env_var.to_string()));
        Ok(())
    }
}

pub struct OsCredentialStore;

impl OsCredentialStore {
    fn entry(server_id: &str, env_var: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(SERVICE_NAME, &format!("{server_id}/{env_var}"))
            .context("open operating-system credential entry")
    }
}

impl CredentialStore for OsCredentialStore {
    fn set(&self, server_id: &str, env_var: &str, value: &str) -> Result<()> {
        Self::entry(server_id, env_var)?
            .set_password(value)
            .context("save MCP credential")
    }

    fn get(&self, server_id: &str, env_var: &str) -> Result<Option<String>> {
        match Self::entry(server_id, env_var)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err).context("read MCP credential"),
        }
    }

    fn delete(&self, server_id: &str, env_var: &str) -> Result<()> {
        match Self::entry(server_id, env_var)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err).context("delete MCP credential"),
        }
    }
}
