use std::collections::BTreeMap;

use anyhow::Result;

use super::credential_store::CredentialStore;
use super::mcp::{credential_name_from_reference, is_credential_name};

/// Resolves `env` references for a managed MCP server before it is rendered
/// into a host configuration: for every credential-named key, the `${NAME}`
/// reference is replaced with the credential value stored by Skills Hub;
/// runtime values (keys that are not credential names) pass through
/// unchanged. The classification matches `validate_mcp_server_input`, which
/// guarantees credential-named keys hold exactly `${NAME}`.
///
/// This runs once at sync time. The resolved (plaintext) environment is
/// written directly into each target app's own configuration file, so MCP
/// servers start without any credential bridge process or Unix socket.
pub fn resolve_credential_environment(
    credentials: &dyn CredentialStore,
    server_id: &str,
    references: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut environment = BTreeMap::new();
    for (name, reference) in references {
        if !is_credential_name(name) {
            environment.insert(name.clone(), reference.clone());
            continue;
        }
        let value = credentials
            .get(server_id, name)?
            .ok_or_else(|| anyhow::anyhow!("missing credential reference {name}"))?;
        environment.insert(name.clone(), value);
    }
    Ok(environment)
}

/// Resolves `headers` values for an HTTP MCP server: every value must be a
/// credential reference (`${NAME}`), replaced with the stored credential
/// value before the header is written into the host configuration.
pub fn resolve_credential_values(
    credentials: &dyn CredentialStore,
    server_id: &str,
    references: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (name, reference) in references {
        let credential_name = credential_name_from_reference(reference)
            .ok_or_else(|| anyhow::anyhow!("invalid credential reference for {name}"))?;
        let value = credentials
            .get(server_id, credential_name)?
            .ok_or_else(|| anyhow::anyhow!("missing credential reference {credential_name}"))?;
        values.insert(name.clone(), value);
    }
    Ok(values)
}
