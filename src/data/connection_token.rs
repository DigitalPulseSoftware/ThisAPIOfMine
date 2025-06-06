use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::{AeadCore, AeadMutInPlace, KeyInit, OsRng};
use deku::prelude::*;
use rand_core::{CryptoRng, RngCore};
use serde::Serialize;
use serde_with::{base64::Base64, serde_as};
use std::mem::size_of;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::deku_helper;
use crate::errors::Result;

use super::player_data::PlayerData;

// size_of will give the correct size of a tag (16)
const XCHACHA20POLY1305_IETF_ABYTES: usize = size_of::<chacha20poly1305::Tag>();
const TOKEN_VERSION: u32 = 2;

#[serde_as]
#[derive(Debug, Serialize)]
struct EncryptionKeys {
    #[serde_as(as = "Base64")]
    client_to_server: chacha20poly1305::Key,
    #[serde_as(as = "Base64")]
    server_to_client: chacha20poly1305::Key,
}

impl EncryptionKeys {
    fn generate<R>(rng: R) -> Self
    where
        R: CryptoRng + RngCore + Copy,
    {
        Self {
            client_to_server: XChaCha20Poly1305::generate_key(rng),
            server_to_client: XChaCha20Poly1305::generate_key(rng),
        }
    }
}

#[derive(Debug, DekuWrite)]
#[deku(endian = "little")]
struct AdditionalTokenData {
    token_version: u32,
    expire_timestamp: u64,
    #[deku(writer = "deku_helper::write_key(deku::writer, &self.client_to_server_key)")]
    client_to_server_key: chacha20poly1305::Key,
    #[deku(writer = "deku_helper::write_key(deku::writer, &self.server_to_client_key)")]
    server_to_client_key: chacha20poly1305::Key,
}

#[serde_as]
#[derive(Debug, Serialize)]
pub struct ConnectionToken<'a> {
    token_version: u32,
    #[serde_as(as = "Base64")]
    token_nonce: chacha20poly1305::XNonce,
    creation_timestamp: u64,
    expire_timestamp: u64,
    encryption_keys: EncryptionKeys,
    game_server: ServerAddress<'a>,
    #[serde_as(as = "Base64")]
    private_token_data: Vec<u8>,
}

impl<'a> ConnectionToken<'a> {
    pub fn generate(
        token_key: &chacha20poly1305::Key,
        duration: Duration,
        server_address: ServerAddress<'a>,
        private_token: PrivateConnectionToken,
    ) -> Result<Self> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let encryption_keys = EncryptionKeys::generate(OsRng);

        let expire_timestamp = timestamp + duration;

        let additional_data = AdditionalTokenData {
            token_version: TOKEN_VERSION,
            expire_timestamp: expire_timestamp.as_secs(),
            client_to_server_key: encryption_keys.client_to_server,
            server_to_client_key: encryption_keys.server_to_client,
        };

        let additional_data_bytes = additional_data.to_bytes()?;

        let nonce = XChaCha20Poly1305::generate_nonce(OsRng);

        let mut private_token_bytes = private_token.to_bytes()?;
        private_token_bytes.resize(private_token_bytes.len() + XCHACHA20POLY1305_IETF_ABYTES, 0);

        let mut cipher = XChaCha20Poly1305::new(token_key);
        cipher.encrypt_in_place(
            &nonce,
            additional_data_bytes.as_slice(),
            &mut private_token_bytes,
        )?;

        Ok(Self {
            token_version: TOKEN_VERSION,
            token_nonce: nonce,
            creation_timestamp: timestamp.as_secs(),
            expire_timestamp: expire_timestamp.as_secs(),
            encryption_keys,
            game_server: server_address,
            private_token_data: private_token_bytes,
        })
    }
}

#[derive(Debug, DekuWrite)]
#[deku(endian = "little")]
pub struct PrivateConnectionToken<'s> {
    #[deku(writer = "deku_helper::write_str(deku::writer, self.refresh_token)")]
    refresh_token: &'s str,
    #[deku(writer = "deku_helper::write_str(deku::writer, self.api_url)")]
    api_url: &'s str,
    player_data: PlayerData,
}

impl<'s> PrivateConnectionToken<'s> {
    pub fn new(api_url: &'s str, refresh_token: &'s str, player_data: PlayerData) -> Self {
        Self {
            refresh_token,
            api_url,
            player_data,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerAddress<'s> {
    address: &'s str,
    port: u16,
}

impl<'s> ServerAddress<'s> {
    pub fn new(address: &'s str, port: u16) -> Self {
        Self { address, port }
    }
}
