use zeroize::Zeroizing;

pub struct ProvisionedHost {
    pub config: Vec<u8>,
    pub security: Zeroizing<Vec<u8>>,
}
