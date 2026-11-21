use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::error::AppError;

const MAGIC: &[u8; 8] = b"DD2BAK01";
const VERSION: u8 = 1;
const KDF_ARGON2ID: u8 = 1;
const CIPHER_XCHACHA: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const HEADER_LEN: usize = 68;
const CHUNK_SIZE: usize = 1024 * 1024;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

const ARGON_M_COST: u32 = 32 * 1024;
const ARGON_T_COST: u32 = 3;
const ARGON_P_COST: u32 = 1;

struct Header {
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    chunk: u32,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
}

fn bad_backup() -> AppError {
    AppError::new("bad_backup", "不是有效的 Diary2Day 备份")
}

fn version_mismatch(version: u8) -> AppError {
    AppError::new(
        "backup_version",
        "备份格式来自更新版本，请先升级 Diary2Day",
    )
    .with_detail(format!("encrypted backup version {version}"))
}

fn bad_password() -> AppError {
    AppError::new("bad_password", "口令不正确，或备份已损坏")
}

pub fn is_encrypted(path: &Path) -> Result<bool, AppError> {
    let mut file = File::open(path)?;
    let mut head = [0u8; 8];
    match file.read_exact(&mut head) {
        Ok(()) => Ok(head == *MAGIC),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(err) => Err(AppError::io(err)),
    }
}

pub fn encrypt(source: &Path, target: &Path, password: &str) -> Result<(), AppError> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut salt)
        .map_err(|err| AppError::internal("随机数生成失败").with_detail(err.to_string()))?;
    getrandom::fill(&mut nonce)
        .map_err(|err| AppError::internal("随机数生成失败").with_detail(err.to_string()))?;

    let header = build_header(&salt, &nonce);
    let key = derive_key(password, &salt, ARGON_M_COST, ARGON_T_COST, ARGON_P_COST)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&*key)
        .map_err(|err| AppError::internal("密钥长度无效").with_detail(err.to_string()))?;

    let mut reader = BufReader::new(File::open(source)?);
    let mut writer = BufWriter::new(File::create(target)?);
    writer.write_all(&header)?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut index: u64 = 0;
    loop {
        let read = read_chunk(&mut reader, &mut buffer)?;
        if read > 0 {
            write_frame(&mut writer, &cipher, &nonce, index, &header, &buffer[..read])?;
            index += 1;
        }
        if read < CHUNK_SIZE {
            break;
        }
    }
    write_frame(&mut writer, &cipher, &nonce, index, &header, &[])?;
    writer.flush()?;
    Ok(())
}

fn write_frame(
    writer: &mut impl Write,
    cipher: &XChaCha20Poly1305,
    nonce: &[u8; NONCE_LEN],
    index: u64,
    aad: &[u8; HEADER_LEN],
    plain: &[u8],
) -> Result<(), AppError> {
    let frame = frame_nonce(nonce, index)?;
    let sealed = cipher
        .encrypt(XNonce::from_slice(&frame), Payload { msg: plain, aad })
        .map_err(|_| AppError::new("backup_crypto", "备份加密失败"))?;
    writer.write_all(&(plain.len() as u32).to_le_bytes())?;
    writer.write_all(&sealed)?;
    Ok(())
}

pub fn decrypt(source: &Path, target: &Path, password: &str) -> Result<(), AppError> {
    let mut reader = BufReader::new(File::open(source)?);
    let mut header_bytes = [0u8; HEADER_LEN];
    reader
        .read_exact(&mut header_bytes)
        .map_err(|_| bad_backup())?;
    let header = parse_header(&header_bytes)?;
    let key = derive_key(
        password,
        &header.salt,
        header.m_cost,
        header.t_cost,
        header.p_cost,
    )?;
    let cipher = XChaCha20Poly1305::new_from_slice(&*key)
        .map_err(|err| AppError::internal("密钥长度无效").with_detail(err.to_string()))?;

    let mut writer = BufWriter::new(File::create(target)?);
    let mut index: u64 = 0;
    loop {
        let mut length_bytes = [0u8; 4];
        reader
            .read_exact(&mut length_bytes)
            .map_err(|_| bad_password())?;
        let length = u32::from_le_bytes(length_bytes) as usize;
        if length > header.chunk as usize {
            return Err(bad_password());
        }
        let mut sealed = vec![0u8; length + TAG_LEN];
        reader.read_exact(&mut sealed).map_err(|_| bad_password())?;
        let frame = frame_nonce(&header.nonce, index)?;
        let plain = cipher
            .decrypt(
                XNonce::from_slice(&frame),
                Payload {
                    msg: &sealed,
                    aad: &header_bytes,
                },
            )
            .map_err(|_| bad_password())?;
        index += 1;
        if length == 0 {
            break;
        }
        writer.write_all(&plain)?;
    }
    writer.flush()?;
    Ok(())
}

fn build_header(salt: &[u8; SALT_LEN], nonce: &[u8; NONCE_LEN]) -> [u8; HEADER_LEN] {
    let mut header = [0u8; HEADER_LEN];
    header[0..8].copy_from_slice(MAGIC);
    header[8] = VERSION;
    header[9] = KDF_ARGON2ID;
    header[10] = CIPHER_XCHACHA;
    header[12..16].copy_from_slice(&ARGON_M_COST.to_le_bytes());
    header[16..20].copy_from_slice(&ARGON_T_COST.to_le_bytes());
    header[20..24].copy_from_slice(&ARGON_P_COST.to_le_bytes());
    header[24..28].copy_from_slice(&(CHUNK_SIZE as u32).to_le_bytes());
    header[28..44].copy_from_slice(salt);
    header[44..68].copy_from_slice(nonce);
    header
}

fn parse_header(bytes: &[u8; HEADER_LEN]) -> Result<Header, AppError> {
    if bytes[..8] != MAGIC[..] {
        return Err(bad_backup());
    }
    if bytes[8] != VERSION {
        return Err(version_mismatch(bytes[8]));
    }
    if bytes[9] != KDF_ARGON2ID || bytes[10] != CIPHER_XCHACHA {
        return Err(AppError::new(
            "backup_format",
            "备份使用了不支持的加密格式",
        ));
    }
    let read_u32 = |range: std::ops::Range<usize>| {
        u32::from_le_bytes(bytes[range].try_into().expect("u32 range"))
    };
    let m_cost = read_u32(12..16);
    let t_cost = read_u32(16..20);
    let p_cost = read_u32(20..24);
    let chunk = read_u32(24..28);
    if !(8 * 1024..=256 * 1024).contains(&m_cost)
        || !(1..=10).contains(&t_cost)
        || !(1..=8).contains(&p_cost)
        || !(4096..=16 * 1024 * 1024).contains(&chunk)
    {
        return Err(AppError::new("backup_format", "备份的加密参数无效"));
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&bytes[28..44]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&bytes[44..68]);
    Ok(Header {
        m_cost,
        t_cost,
        p_cost,
        chunk,
        salt,
        nonce,
    })
}

fn derive_key(
    password: &str,
    salt: &[u8; SALT_LEN],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, AppError> {
    let params = Params::new(m_cost, t_cost, p_cost, Some(KEY_LEN))
        .map_err(|err| AppError::internal("加密参数无效").with_detail(err.to_string()))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password.as_bytes(), salt, &mut *key)
        .map_err(|err| AppError::internal("密钥派生失败").with_detail(err.to_string()))?;
    Ok(key)
}

fn frame_nonce(base: &[u8; NONCE_LEN], index: u64) -> Result<[u8; NONCE_LEN], AppError> {
    let mut nonce = *base;
    let base_tail = u64::from_be_bytes(nonce[16..24].try_into().expect("u64 range"));
    let next = base_tail
        .checked_add(index)
        .ok_or_else(|| AppError::internal("备份分块计数溢出"))?;
    nonce[16..24].copy_from_slice(&next.to_be_bytes());
    Ok(nonce)
}

fn read_chunk(reader: &mut impl Read, buffer: &mut [u8]) -> Result<usize, AppError> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(AppError::io(err)),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    const PASSWORD: &str = "correct horse battery";

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let unique = format!(
                "diary2day-backup-crypt-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|index| (index % 251) as u8).collect()
    }

    fn round_trip(plain: &[u8]) {
        let dir = TempDir::new();
        let source = dir.file("payload.zip");
        let encrypted = dir.file("backup.ddbak");
        let restored = dir.file("restored.zip");
        std::fs::write(&source, plain).unwrap();

        encrypt(&source, &encrypted, PASSWORD).unwrap();
        assert!(is_encrypted(&encrypted).unwrap());
        assert!(!is_encrypted(&source).unwrap());

        decrypt(&encrypted, &restored, PASSWORD).unwrap();
        assert_eq!(std::fs::read(&restored).unwrap(), plain);
    }

    #[test]
    fn round_trips_empty_payload() {
        round_trip(b"");
    }

    #[test]
    fn round_trips_small_payload() {
        round_trip(b"PK\x03\x04 diary2day payload");
    }

    #[test]
    fn round_trips_exact_chunk() {
        round_trip(&pattern(CHUNK_SIZE));
    }

    #[test]
    fn round_trips_multiple_chunks() {
        round_trip(&pattern(CHUNK_SIZE * 2 + 12345));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let dir = TempDir::new();
        let source = dir.file("payload.zip");
        let encrypted = dir.file("backup.ddbak");
        let restored = dir.file("restored.zip");
        std::fs::write(&source, pattern(64)).unwrap();
        encrypt(&source, &encrypted, PASSWORD).unwrap();

        let err = decrypt(&encrypted, &restored, "another password").unwrap_err();
        assert_eq!(err.code, "bad_password");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let dir = TempDir::new();
        let source = dir.file("payload.zip");
        let encrypted = dir.file("backup.ddbak");
        let tampered = dir.file("tampered.ddbak");
        let restored = dir.file("restored.zip");
        std::fs::write(&source, pattern(64)).unwrap();
        encrypt(&source, &encrypted, PASSWORD).unwrap();

        let mut bytes = std::fs::read(&encrypted).unwrap();
        bytes[HEADER_LEN + 4] ^= 0x01;
        std::fs::write(&tampered, bytes).unwrap();

        let err = decrypt(&tampered, &restored, PASSWORD).unwrap_err();
        assert_eq!(err.code, "bad_password");
    }

    #[test]
    fn truncated_stream_is_rejected() {
        let dir = TempDir::new();
        let source = dir.file("payload.zip");
        let encrypted = dir.file("backup.ddbak");
        let truncated = dir.file("truncated.ddbak");
        let restored = dir.file("restored.zip");
        std::fs::write(&source, pattern(64)).unwrap();
        encrypt(&source, &encrypted, PASSWORD).unwrap();

        let bytes = std::fs::read(&encrypted).unwrap();
        std::fs::write(&truncated, &bytes[..bytes.len() - 5]).unwrap();

        let err = decrypt(&truncated, &restored, PASSWORD).unwrap_err();
        assert_eq!(err.code, "bad_password");
    }

    #[test]
    fn plain_zip_is_not_treated_as_encrypted() {
        let dir = TempDir::new();
        let plain = dir.file("plain.zip");
        std::fs::write(&plain, b"PK\x03\x04 not a ddbak file").unwrap();
        assert!(!is_encrypted(&plain).unwrap());
    }
}