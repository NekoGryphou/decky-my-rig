use myrig_host_core::{
    paired_devices::RegistryConfig, provisioning::generate, security_bundle::HostSecurity,
};

#[test]
fn new_host_has_matching_tls_identity_and_empty_registry() {
    let provisioned = generate(47991).expect("generate new host");
    let config = RegistryConfig::from_json(&provisioned.config).expect("schema-3 configuration");
    assert_eq!(config.port(), 47991);
    assert!(!config.test_mode());
    assert!(config.public_summary().paired_devices.is_empty());
    let identity = HostSecurity::from_trusted_json(&provisioned.security, config.host_id())
        .expect("matching TLS identity")
        .into_identity();
    assert!(!identity.leaf_der().is_empty());
}

#[test]
fn provisioning_rejects_zero_port() {
    assert!(generate(0).is_err());
}
