use std::fmt;

/// Realm join key material: 32 client bytes followed by 32 server bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RealmJoinSecrets([u8; 64]);

impl RealmJoinSecrets {
    pub fn new(client_secret: [u8; 32], server_secret: [u8; 32]) -> Self {
        let mut data = [0u8; 64];

        data[..32].copy_from_slice(&client_secret);
        data[32..].copy_from_slice(&server_secret);

        Self(data)
    }

    pub const fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub const fn into_bytes(self) -> [u8; 64] {
        self.0
    }
}

impl From<[u8; 64]> for RealmJoinSecrets {
    fn from(data: [u8; 64]) -> Self {
        Self(data)
    }
}

impl fmt::Debug for RealmJoinSecrets {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RealmJoinSecrets([REDACTED])")
    }
}
