use std::collections::BTreeMap;

use anyhow::Result;

use super::credential_store::CredentialStore;

pub fn resolve_bridge_environment(
    credentials: &dyn CredentialStore,
    server_id: &str,
    references: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut environment = BTreeMap::new();
    for (name, reference) in references {
        let expected = format!("${{{name}}}");
        if reference != &expected {
            anyhow::bail!("invalid credential reference for {name}");
        }
        let value = credentials
            .get(server_id, name)?
            .ok_or_else(|| anyhow::anyhow!("missing credential reference {name}"))?;
        environment.insert(name.clone(), value);
    }
    Ok(environment)
}
