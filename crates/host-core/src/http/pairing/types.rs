/// A parsed pairing finish request.
pub(super) struct FinishRequest {
    pub(super) attempt: [u8; 32],
    pub(super) confirmation: [u8; myrig_pairing_crypto::CONFIRMATION_LEN],
    pub(super) previous: Option<zeroize::Zeroizing<String>>,
}
