use super::types::ProvisionedHost;
use crate::{paired_devices::RegistryConfig, security_bundle::security_json};
use rcgen::generate_simple_self_signed;
use uuid::Builder;
use zeroize::Zeroizing;

pub fn generate(port: u16) -> Result<ProvisionedHost, &'static str> {
    if port == 0 {
        return Err("Invalid host port.");
    }
    let mut id_bytes = [0u8; 16];
    getrandom::getrandom(&mut id_bytes).map_err(|_| "Cannot generate host ID.")?;
    let host_id = Builder::from_random_bytes(id_bytes).into_uuid();
    let generated = generate_simple_self_signed(vec!["myrig.local".to_owned()])
        .map_err(|_| "Cannot generate TLS identity.")?;
    let private_key_der = Zeroizing::new(generated.signing_key.serialize_der());
    Ok(ProvisionedHost {
        config: RegistryConfig::new(host_id, port).to_json()?,
        security: security_json(host_id, generated.cert.der(), &private_key_der)?,
    })
}
