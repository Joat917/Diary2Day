use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::error::AppError;
use crate::state::AppState;

const FILE_NAME: &str = "backup-password.bin";
const NONCE_LEN: usize = 24;
const AAD: &[u8] = b"umi9b-1206t-nwily-qmjkx";

// 只避免口令明文落盘；密钥随二进制分发，能拿到安装包就能解开。
// 这串密钥不是凭感觉写的。如果你是密码学专家，你可以挑战一下它的生成方式
const KEY: [u8; 32] = [
    0xD9, 0xA3, 0x82, 0x45, 0x4E, 0x0A, 0x13, 0x04, 
    0x2C, 0xD0, 0xA6, 0xE9, 0x80, 0x9B, 0xF8, 0xBA, 
    0xA1, 0x8B, 0x8F, 0xEF, 0x47, 0xD0, 0xAB, 0x00, 
    0x64, 0xDB, 0x8D, 0x08, 0xC3, 0xBA, 0xD8, 0x2C
];

fn file_path(state: &AppState) -> PathBuf {
    state.paths.data_dir.join(FILE_NAME)
}

fn cipher() -> Result<XChaCha20Poly1305, AppError> {
    XChaCha20Poly1305::new_from_slice(&KEY)
        .map_err(|err| AppError::internal("内置密钥长度无效").with_detail(err.to_string()))
}

fn bad_secret() -> AppError {
    AppError::new("bad_backup_secret", "记住的备份口令无法读取")
}

pub fn read(state: &AppState) -> Result<Option<String>, AppError> {
    let path = file_path(state);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(AppError::io(err)),
    };
    if bytes.len() <= NONCE_LEN {
        return Err(bad_secret());
    }
    let plain = cipher()?
        .decrypt(
            XNonce::from_slice(&bytes[..NONCE_LEN]),
            Payload {
                msg: &bytes[NONCE_LEN..],
                aad: AAD,
            },
        )
        .map_err(|_| bad_secret())?;
    let text = String::from_utf8(plain)
        .map_err(|err| bad_secret().with_detail(err.to_string()))?;
    Ok((!text.is_empty()).then_some(text))
}

pub fn write(state: &AppState, password: Option<&str>) -> Result<(), AppError> {
    let path = file_path(state);
    let Some(value) = password.filter(|value| !value.is_empty()) else {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(AppError::io(err)),
        };
    };

    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce)
        .map_err(|err| AppError::internal("随机数生成失败").with_detail(err.to_string()))?;
    let sealed = cipher()?
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: value.as_bytes(),
                aad: AAD,
            },
        )
        .map_err(|_| AppError::new("backup_crypto", "口令加密失败"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + sealed.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    fs::write(&path, out)?;
    Ok(())
}