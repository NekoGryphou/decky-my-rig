use myrig_host_core::{paired_devices::RegistryConfig, security_bundle::HostSecurity};
use myrig_setup::initialize;

#[test]
fn first_install_creates_matching_files_and_never_reads_or_replaces_them() {
    let root = tempfile::tempdir().expect("temporary install root");
    let config = root.path().join("host.json");
    let security = root.path().join("host-security.json");
    initialize(&config, &security, 47991).unwrap();
    let config_before = std::fs::read(&config).unwrap();
    let security_before = std::fs::read(&security).unwrap();
    let host = RegistryConfig::from_json(&config_before).unwrap();
    HostSecurity::from_trusted_json(&security_before, host.host_id()).unwrap();
    // - Upgrades never run this: existing files are refused, not checked.
    assert!(initialize(&config, &security, 47992).is_err());
    assert_eq!(std::fs::read(&config).unwrap(), config_before);
    assert_eq!(std::fs::read(&security).unwrap(), security_before);
}

#[test]
fn partial_install_is_refused_without_replacing_existing_file() {
    let root = tempfile::tempdir().expect("temporary install root");
    let config = root.path().join("host.json");
    let security = root.path().join("host-security.json");
    std::fs::write(&config, b"existing").unwrap();
    assert!(initialize(&config, &security, 47991).is_err());
    assert_eq!(std::fs::read(&config).unwrap(), b"existing");
    assert!(!security.exists());
}

#[test]
fn a_taken_port_moves_to_the_next_free_one() {
    let taken = std::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let port = taken.local_addr().unwrap().port();
    let chosen = myrig_setup::first_free_port(port).unwrap();
    assert!(chosen > port);
    drop(taken);
    assert_eq!(myrig_setup::first_free_port(port).unwrap(), port);
}

#[test]
fn the_port_probe_never_listens() {
    // ? A probe that listens makes Windows Firewall prompt for the setup helper
    //   and keep an inbound rule for it.
    let probe = myrig_setup::probe_port(0).unwrap();
    let port = probe.local_addr().unwrap().as_socket().unwrap().port();
    assert!(std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_err());
}
