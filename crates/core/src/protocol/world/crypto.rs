use aes_gcm::{
    AesGcm, Nonce, Tag,
    aead::{AeadInPlace, KeyInit, consts::U12},
    aes::Aes256,
};
use anyhow::{Result, anyhow, ensure};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha512};
use subtle::ConstantTimeEq;

use crate::sessions::{RealmJoinSecrets, WorldSessionKey};

type WorldAes = AesGcm<Aes256, U12, U12>;

const SESSION_SEED: [u8; 32] = [
    0xe8, 0x1e, 0x8b, 0x59, 0x27, 0x62, 0x1e, 0xaa,
    0x86, 0x15, 0x18, 0xea, 0xc0, 0xbf, 0x66, 0x8c,
    0x6d, 0xbf, 0x83, 0x93, 0xbc, 0xaa, 0x80, 0x52,
    0x5b, 0x1e, 0xdc, 0x23, 0xa0, 0x12, 0xb7, 0x50,
];

const CONTINUED_SEED: [u8; 32] = [
    0x56, 0x5c, 0x61, 0x9c, 0x48, 0x3a, 0x52, 0x1f,
    0x61, 0x5d, 0x05, 0x49, 0xb2, 0x9a, 0x39, 0xbf,
    0x4b, 0x97, 0xb0, 0x1b, 0xf9, 0x6c, 0xde, 0xd6,
    0x80, 0x1d, 0xab, 0x26, 0x02, 0xa9, 0x9b, 0x9d,
];

const ENCRYPTION_SEED: [u8; 32] = [
    0x71, 0xc9, 0xed, 0x5a, 0xa7, 0x0e, 0x4d, 0xff,
    0x4c, 0x36, 0xa6, 0x5a, 0x3e, 0x46, 0x8a, 0x4a,
    0x5d, 0xa1, 0x48, 0xc8, 0x30, 0x47, 0x4a, 0xde,
    0xf6, 0x0d, 0x6c, 0xbe, 0x6f, 0xe4, 0x55, 0x73,
];

fn mac(key: &[u8], parts: &[&[u8]]) -> [u8; 64] {
    let mut state =
        <Hmac<Sha512> as Mac>::new_from_slice(key)
            .expect("HMAC accepts arbitrary key lengths");

    for part in parts {
        state.update(part);
    }

    state.finalize().into_bytes().into()
}

pub(super) fn derive_session(
    secrets: &RealmJoinSecrets,
    server: &[u8; 32],
    client: &[u8; 32],
) -> WorldSessionKey {
    let key_hash = Sha512::digest(secrets.as_bytes());

    let seed = mac(
        &key_hash,
        &[server, client, &SESSION_SEED],
    );

    let left = Sha512::digest(&seed[..32]);
    let right = Sha512::digest(&seed[32..]);

    let mut generator = Sha512::new();
    generator.update(left);
    generator.update([0u8; 64]);
    generator.update(right);

    let generated = generator.finalize();
    let mut session = [0u8; 40];
    session.copy_from_slice(&generated[..40]);

    WorldSessionKey::new(session)
}

fn derive_encryption(
    session: &WorldSessionKey,
    server: &[u8; 32],
    client: &[u8; 32],
) -> [u8; 32] {
    let generated = mac(
        session.as_bytes(),
        &[client, server, &ENCRYPTION_SEED],
    );

    let mut key = [0u8; 32];
    key.copy_from_slice(&generated[..32]);
    key
}

pub(super) fn verify_continued(
    session: &WorldSessionKey,
    server: &[u8; 32],
    client: &[u8; 32],
    connection_key: &[u8; 8],
    proof: &[u8; 24],
) -> bool {
    let expected = mac(
        session.as_bytes(),
        &[connection_key, client, server, &CONTINUED_SEED],
    );

    bool::from(expected[..24].ct_eq(proof))
}

fn nonce(from_client: bool, counter: u64) -> Nonce<U12> {
    let mut value = [0u8; 12];

    value[..8].copy_from_slice(&counter.to_le_bytes());
    value[8..].copy_from_slice(
        if from_client { b"CLNT" } else { b"SRVR" },
    );

    value.into()
}

#[derive(Clone)]
pub struct KeyCandidate {
    session: WorldSessionKey,
    cipher: WorldAes,
}

impl KeyCandidate {
    pub(super) fn new(
        session: WorldSessionKey,
        server: &[u8; 32],
        client: &[u8; 32],
    ) -> Self {
        let key = derive_encryption(&session, server, client);

        Self {
            session,
            cipher: WorldAes::new(&key.into()),
        }
    }

    pub fn session(&self) -> WorldSessionKey {
        self.session
    }

    fn decrypt(
        &self,
        from_client: bool,
        counter: u64,
        tag: &[u8; 12],
        data: &[u8],
    ) -> Result<Bytes> {
        let mut plain = data.to_vec();
        let tag: Tag<U12> = (*tag).into();

        self.cipher
            .decrypt_in_place_detached(
                &nonce(from_client, counter),
                &[],
                &mut plain,
                &tag,
            )
            .map_err(|_| {
                anyhow!(
                    "World AES-GCM verification failed: \
                     from_client={from_client}, counter={counter}",
                )
            })?;

        Ok(Bytes::from(plain))
    }
}

#[derive(Default)]
pub(super) struct CryptoState {
    candidates: Vec<KeyCandidate>,
    selected: Option<KeyCandidate>,
}

impl CryptoState {
    pub fn bind(&mut self, candidates: Vec<KeyCandidate>) -> Result<()> {
        ensure!(
            self.selected.is_none() && self.candidates.is_empty(),
            "World keys are already bound",
        );

        ensure!(
            !candidates.is_empty(),
            "No World encryption candidates",
        );

        self.candidates = candidates;
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.selected.is_some() || !self.candidates.is_empty()
    }

    pub fn decrypt(
        &mut self,
        from_client: bool,
        counter: u64,
        tag: &[u8; 12],
        data: &[u8],
    ) -> Result<(Bytes, Option<WorldSessionKey>)> {
        if let Some(selected) = &self.selected {
            return Ok((
                selected.decrypt(from_client, counter, tag, data)?,
                None,
            ));
        }

        for candidate in &self.candidates {
            if let Ok(plain) =
                candidate.decrypt(from_client, counter, tag, data)
            {
                let selected = candidate.clone();
                let session = selected.session;

                self.selected = Some(selected);
                self.candidates.clear();

                return Ok((plain, Some(session)));
            }
        }

        Err(anyhow!(
            "No World key passed AES-GCM verification: \
             from_client={from_client}, counter={counter}",
        ))
    }

    pub fn encrypt(
        &self,
        from_client: bool,
        counter: u64,
        data: &[u8],
    ) -> Result<([u8; 12], Bytes)> {
        let selected = self
            .selected
            .as_ref()
            .ok_or_else(|| anyhow!("World encryption key is not verified"))?;

        let mut ciphertext = data.to_vec();

        let tag = selected
            .cipher
            .encrypt_in_place_detached(
                &nonce(from_client, counter),
                &[],
                &mut ciphertext,
            )
            .map_err(|_| anyhow!("World AES-GCM encryption failed"))?;

        Ok((tag.into(), Bytes::from(ciphertext)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn matches_python_key_derivation_and_gcm() {
        let raw: [u8; 64] = std::array::from_fn(|index| index as u8);
        let server = std::array::from_fn(|index| index as u8);
        let client = std::array::from_fn(|index| (index + 32) as u8);

        let session = derive_session(
            &RealmJoinSecrets::from(raw),
            &server,
            &client,
        );

        assert_eq!(
            hex(session.as_bytes()),
            "069059cb2323a7ff09300360c4874c6913936967bc91f677ce9ebe8bf523b839e7ada2c524611957",
        );

        assert_eq!(
            hex(&derive_encryption(&session, &server, &client)),
            "a916f3321fd19b5c218314066feb6e2ffe32aeecdb95f7b10484c0996a69a279",
        );

        let mut crypto = CryptoState::default();
        let candidate = KeyCandidate::new(session, &server, &client);

        let mut body = 0x4501b7u32.to_le_bytes().to_vec();
        body.extend(0u8..16);

        crypto.selected = Some(candidate.clone());

        let (tag, ciphertext) = crypto.encrypt(false, 5, &body).unwrap();

        assert_eq!(hex(&tag), "aebdc8eda26a726cc4d5bbd7");
        assert_eq!(
            hex(&ciphertext),
            "45373fda7d7b30893a7da13da7037d109abe161d",
        );

        assert_eq!(
            candidate.decrypt(false, 5, &tag, &ciphertext).unwrap(),
            Bytes::from(body),
        );

        let mut invalid = tag;
        invalid[0] ^= 1;

        assert!(
            candidate.decrypt(false, 5, &invalid, &ciphertext).is_err(),
        );
    }
}
