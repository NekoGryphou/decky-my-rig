use super::{
    consts::{MAX_CONFIG_BYTES, MAX_PAIRED_DEVICES, SCHEMA_VERSION},
    types::{
        PairedDevice, PublicPairedDevice, PublicRegistryConfig, RegistryConfig, StoredConfig,
        StoredDevice,
    },
    utils::{digests_match, token_digest},
};
use myrig_protocol::validation::{canonical_uuid, decode_hex, lower_hex, valid_name};
use uuid::Uuid;

impl RegistryConfig {
    /// Config for a new host: no paired devices, test mode off.
    pub fn new(host_id: Uuid, port: u16) -> Self {
        Self {
            host_id,
            port,
            test_mode: false,
            paired_devices: Vec::new(),
        }
    }

    pub fn insert_credential(
        &mut self,
        device_id: Uuid,
        name: &str,
        digest: [u8; 32],
    ) -> Result<(), &'static str> {
        if !valid_name(name) {
            return Err("Invalid device name.");
        }
        if self.paired_devices.len() >= MAX_PAIRED_DEVICES {
            return Err("Too many paired devices.");
        }
        if self
            .paired_devices
            .iter()
            .any(|device| device.device_id == device_id || device.token_sha256 == digest)
        {
            return Err("Duplicate paired device credential or ID.");
        }
        self.paired_devices.push(PairedDevice {
            device_id,
            name: name.to_owned(),
            token_sha256: digest,
        });
        Ok(())
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_CONFIG_BYTES {
            return Err("Host configuration exceeds size limit.");
        }
        let stored: StoredConfig =
            serde_json::from_slice(bytes).map_err(|_| "Invalid host configuration.")?;
        if stored.schema_version != SCHEMA_VERSION {
            return Err("Unsupported host configuration schema.");
        }
        let host_id = canonical_uuid(&stored.host_id).ok_or("Invalid host ID.")?;
        if stored.port == 0 {
            return Err("Invalid host port.");
        }
        let mut registry = Self::new(host_id, stored.port);
        registry.test_mode = stored.test_mode;
        for device in stored.paired_devices {
            let device_id = canonical_uuid(&device.device_id).ok_or("Invalid device ID.")?;
            let token_sha256 = decode_hex(&device.token_sha256).ok_or("Invalid token digest.")?;
            registry.insert_credential(device_id, &device.name, token_sha256)?;
        }
        Ok(registry)
    }

    pub fn authenticate_device(&self, token: &str) -> Option<String> {
        self.device_index(token)
            .map(|index| self.paired_devices[index].name.clone())
    }

    /// Revokes the credential `token` authenticates, if any.
    pub fn revoke_token(&mut self, token: &str) -> bool {
        self.device_index(token)
            .map(|index| self.paired_devices.remove(index))
            .is_some()
    }

    /// Compares against every credential slot in constant time before picking
    /// the matching device.
    fn device_index(&self, token: &str) -> Option<usize> {
        let actual = token_digest(token)?;
        let mut matched = [false; MAX_PAIRED_DEVICES];
        for (index, slot) in matched.iter_mut().enumerate() {
            let entry = self.paired_devices.get(index);
            let digest = entry.map_or(&[0; 32], |device| &device.token_sha256);
            *slot = digests_match(&actual, digest) & entry.is_some();
        }
        matched.iter().position(|matched| *matched)
    }

    pub fn test_mode(&self) -> bool {
        self.test_mode
    }

    pub fn set_test_mode(&mut self, enabled: bool) -> bool {
        if self.test_mode == enabled {
            return false;
        }
        self.test_mode = enabled;
        true
    }

    /// Changes the saved port. The caller rebinds the listener once it's saved.
    pub fn set_port(&mut self, port: u16) -> bool {
        if port == 0 || port == self.port {
            return false;
        }
        self.port = port;
        true
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn host_id(&self) -> Uuid {
        self.host_id
    }

    /// False if the device is unknown or already has `name`. The caller
    /// validates the name.
    pub fn rename(&mut self, device_id: Uuid, name: &str) -> bool {
        match self
            .paired_devices
            .iter_mut()
            .find(|device| device.device_id == device_id)
        {
            Some(device) if device.name != name => {
                name.clone_into(&mut device.name);
                true
            }
            _ => false,
        }
    }

    pub fn revoke(&mut self, device_id: Uuid) -> bool {
        let before = self.paired_devices.len();
        self.paired_devices
            .retain(|device| device.device_id != device_id);
        self.paired_devices.len() != before
    }

    pub fn public_summary(&self) -> PublicRegistryConfig {
        PublicRegistryConfig {
            schema_version: SCHEMA_VERSION,
            host_id: self.host_id.to_string(),
            port: self.port,
            test_mode: self.test_mode,
            paired_devices: self
                .paired_devices
                .iter()
                .map(|device| PublicPairedDevice {
                    device_id: device.device_id.to_string(),
                    name: device.name.clone(),
                })
                .collect(),
        }
    }

    /// The bytes to save in protected storage. They contain token digests,
    /// never a raw token.
    pub fn to_json(&self) -> Result<Vec<u8>, &'static str> {
        let stored = StoredConfig {
            schema_version: SCHEMA_VERSION,
            host_id: self.host_id.to_string(),
            port: self.port,
            test_mode: self.test_mode,
            paired_devices: self
                .paired_devices
                .iter()
                .map(|device| StoredDevice {
                    device_id: device.device_id.to_string(),
                    name: device.name.clone(),
                    token_sha256: lower_hex(&device.token_sha256),
                })
                .collect(),
        };
        let bytes =
            serde_json::to_vec(&stored).map_err(|_| "Cannot serialize host configuration.")?;
        if bytes.len() > MAX_CONFIG_BYTES {
            return Err("Host configuration exceeds size limit.");
        }
        Ok(bytes)
    }
}
