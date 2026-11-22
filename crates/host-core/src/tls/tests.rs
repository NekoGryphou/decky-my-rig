use super::utils::server_config_from_der;
use super::*;
#[test]
fn invalid_identity_is_rejected_without_fallback() {
    assert!(server_config_from_der(vec![], vec![]).is_err());
    assert!(server_config_from_der(vec![vec![1, 2, 3]], vec![1, 2, 3]).is_err());
    assert!(FixedIdentity::from_der(vec![], vec![]).is_err());
    assert!(FixedIdentity::from_der(vec![vec![1, 2, 3]], vec![1, 2, 3]).is_err());
}
