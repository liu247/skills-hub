use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use aes_gcm::aead::{rand_core::RngCore, Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};

use super::skill_store::SkillStore;

const SERVICE_NAME: &str = "skills-hub.mcp";
const LOCAL_CREDENTIAL_PREFIX: &str = "credential.v1.";
const LOCAL_KEY_FILE_NAME: &str = "credentials.key";

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

/// An update-stable credential store: encrypted values live in the app database,
/// while its random encryption key stays in the app data directory (outside the app bundle).
#[derive(Clone)]
pub struct LocalCredentialStore {
    store: SkillStore,
    key_path: PathBuf,
}

impl LocalCredentialStore {
    pub fn from_store(store: &SkillStore) -> Result<Self> {
        let parent = store
            .db_path()
            .parent()
            .context("resolve local credential directory")?;
        std::fs::create_dir_all(parent).context("create local credential directory")?;
        Ok(Self {
            store: store.clone(),
            key_path: parent.join(LOCAL_KEY_FILE_NAME),
        })
    }

    pub(crate) fn setting_key(server_id: &str, env_var: &str) -> String {
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(server_id.as_bytes());
        hasher.update([0]);
        hasher.update(env_var.as_bytes());
        format!(
            "{LOCAL_CREDENTIAL_PREFIX}{}",
            hex::encode(hasher.finalize())
        )
    }

    fn load_key(&self) -> Result<[u8; 32]> {
        match File::open(&self.key_path) {
            Ok(mut file) => {
                let mut key = [0_u8; 32];
                file.read_exact(&mut key)
                    .context("read local credential key")?;
                let mut trailing = [0_u8; 1];
                if file.read(&mut trailing)? != 0 {
                    anyhow::bail!("local credential key has invalid length");
                }
                Ok(key)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => self.create_key(),
            Err(error) => Err(error).context("open local credential key"),
        }
    }

    fn create_key(&self) -> Result<[u8; 32]> {
        let mut key = [0_u8; 32];
        OsRng.fill_bytes(&mut key);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.key_path)
        {
            Ok(mut file) => {
                file.write_all(&key).context("write local credential key")?;
                file.sync_all().context("sync local credential key")?;
                #[cfg(unix)]
                std::fs::set_permissions(&self.key_path, std::fs::Permissions::from_mode(0o600))
                    .context("secure local credential key")?;
                Ok(key)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => self.load_key(),
            Err(error) => Err(error).context("create local credential key"),
        }
    }

    fn crypt(
        &self,
        server_id: &str,
        env_var: &str,
        value: &[u8],
        encrypt: bool,
    ) -> Result<Vec<u8>> {
        let key = self.load_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key).context("create local credential cipher")?;
        let aad = format!("{server_id}\0{env_var}");
        if encrypt {
            let mut nonce = [0_u8; 12];
            OsRng.fill_bytes(&mut nonce);
            let mut output = nonce.to_vec();
            output.extend(
                cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        aes_gcm::aead::Payload {
                            msg: value,
                            aad: aad.as_bytes(),
                        },
                    )
                    .map_err(|_| anyhow::anyhow!("encrypt local credential"))?,
            );
            Ok(output)
        } else {
            if value.len() < 13 {
                anyhow::bail!("encrypted local credential is invalid");
            }
            cipher
                .decrypt(
                    Nonce::from_slice(&value[..12]),
                    aes_gcm::aead::Payload {
                        msg: &value[12..],
                        aad: aad.as_bytes(),
                    },
                )
                .map_err(|_| anyhow::anyhow!("decrypt local credential"))
        }
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

impl CredentialStore for LocalCredentialStore {
    fn set(&self, server_id: &str, env_var: &str, value: &str) -> Result<()> {
        let encrypted = self.crypt(server_id, env_var, value.as_bytes(), true)?;
        self.store.set_setting(
            &Self::setting_key(server_id, env_var),
            &hex::encode(encrypted),
        )
    }

    fn get(&self, server_id: &str, env_var: &str) -> Result<Option<String>> {
        let Some(value) = self
            .store
            .get_setting(&Self::setting_key(server_id, env_var))?
        else {
            return Ok(None);
        };
        let encrypted = hex::decode(value).context("decode encrypted local credential")?;
        let plaintext = self.crypt(server_id, env_var, &encrypted, false)?;
        String::from_utf8(plaintext)
            .map(Some)
            .context("decode local credential")
    }

    fn delete(&self, server_id: &str, env_var: &str) -> Result<()> {
        self.store
            .delete_setting(&Self::setting_key(server_id, env_var))
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
