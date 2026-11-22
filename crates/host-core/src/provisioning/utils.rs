use super::types::ProvisionedHost;
use crate::{paired_devices::RegistryConfig, security_bundle::security_json};
use rcgen::generate_simple_self_signed;
use uuid::Uuid;
use zeroize::Zeroizing;

pub fn generate(port: u16) -> Result<ProvisionedHost, &'static str> {
    if port == 0 {
        return Err("Invalid host port.");
    }
    let mut id_bytes = [0u8; 16];
    getrandom::getrandom(&mut id_bytes).map_err(|_| "Cannot generate host ID.")?;
    id_bytes[6] = (id_bytes[6] & 0x0f) | 0x40;
    id_bytes[8] = (id_bytes[8] & 0x3f) | 0x80;
    let host_id = Uuid::from_bytes(id_bytes);
    let generated = generate_simple_self_signed(vec!["myrig.local".to_owned()])
        .map_err(|_| "Cannot generate TLS identity.")?;
    let private_key_der = Zeroizing::new(generated.signing_key.serialize_der());
    Ok(ProvisionedHost {
        config: RegistryConfig::new(host_id, port).to_json()?,
        security: security_json(host_id, generated.cert.der(), &private_key_der)?,
    })
}
