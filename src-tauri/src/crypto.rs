use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use aes_gcm::aead::OsRng;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};

use crate::errors::{AppError, AppResult};

/// 密钥派生参数
const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// 历史"前端固定口令"层的口令，**仅**用于解封旧数据（见
/// `db::Database::migrate_legacy_secret`），新数据一律走本地 keystore。
/// 设置页保存链路已不再使用该口令（曾导致加密形同虚设 + LLM 拿到坏 Key）。
pub const LEGACY_SETTING_PASSWORD: &str = "ace-default-password";

/// 从密码派生 AES-256 密钥
fn derive_key(password: &[u8], salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password, salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// 加密 API Key
/// 返回格式: base64(salt + nonce + ciphertext)
pub fn encrypt_api_key(api_key: &str, password: &str) -> AppResult<String> {
    // 生成随机 salt 和 nonce
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    // 派生密钥
    let key = derive_key(password.as_bytes(), &salt);

    // 创建 cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AppError::Crypto(format!("创建cipher失败: {}", e)))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    // 加密
    let ciphertext = cipher.encrypt(nonce, api_key.as_bytes())
        .map_err(|e| AppError::Crypto(format!("加密失败: {}", e)))?;

    // 组合: salt + nonce + ciphertext
    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    // Base64 编码
    Ok(general_purpose::STANDARD.encode(&result))
}

/// 解密 API Key
pub fn decrypt_api_key(encrypted: &str, password: &str) -> AppResult<String> {
    // Base64 解码
    let data = general_purpose::STANDARD.decode(encrypted)
        .map_err(|e| AppError::Crypto(format!("Base64解码失败: {}", e)))?;

    if data.len() < SALT_LEN + NONCE_LEN {
        return Err(AppError::Crypto("加密数据格式无效".to_string()));
    }

    // 提取 salt, nonce, ciphertext
    let (salt, rest) = data.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    // 派生密钥
    let key = derive_key(password.as_bytes(), salt);

    // 创建 cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AppError::Crypto(format!("创建cipher失败: {}", e)))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    // 解密
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Crypto(format!("解密失败: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Crypto(format!("UTF-8转换失败: {}", e)))
}

/// 验证加密格式是否有效
pub fn is_encrypted_format(data: &str) -> bool {
    if let Ok(decoded) = general_purpose::STANDARD.decode(data) {
        decoded.len() > SALT_LEN + NONCE_LEN
    } else {
        false
    }
}

// ==================== 本地密钥加密（用于敏感设置项静态加密） ====================

/// 本地密钥文件名（与数据库同目录）
pub const KEYSTORE_FILE: &str = "keystore.key";

/// 本模块产生的密文前缀，用于区分密文与历史明文
pub const SECRET_PREFIX: &str = "enc:v1:";

/// keystore 落盘封装格式魔数（Windows：魔数 + DPAPI 密文；Unix：裸 32 字节 + 0600）
#[cfg(windows)]
const KEYSTORE_MAGIC: &[u8; 8] = b"ACEKST1\0";

/// Windows 落盘保护：调用 DPAPI（CryptProtectData / CryptUnprotectData），
/// 把 keystore 绑定到当前 Windows 用户。仅拷走应用数据目录（数据库 + keystore）
/// 而不带用户凭据时无法解密，堵住"整目录被复制即失守"的缺口。
#[cfg(windows)]
mod at_rest {
    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            data_in: *const DataBlob,
            data_descr: *const u16,
            optional_entropy: *const u16,
            reserved: *mut core::ffi::c_void,
            prompt_struct: *mut core::ffi::c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            data_in: *const DataBlob,
            data_descr: *mut *mut u16,
            optional_entropy: *const u16,
            reserved: *mut core::ffi::c_void,
            prompt_struct: *mut core::ffi::c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    unsafe fn blob_to_vec(blob: &DataBlob) -> Vec<u8> {
        if blob.pb_data.is_null() || blob.cb_data == 0 {
            return Vec::new();
        }
        let out = std::slice::from_raw_parts(blob.pb_data, blob.cb_data as usize).to_vec();
        LocalFree(blob.pb_data as *mut core::ffi::c_void);
        out
    }

    pub fn protect(plain: &[u8]) -> std::io::Result<Vec<u8>> {
        let input = DataBlob {
            cb_data: plain.len() as u32,
            pb_data: plain.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { blob_to_vec(&output) })
    }

    pub fn unprotect(data: &[u8]) -> std::io::Result<Vec<u8>> {
        let input = DataBlob {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { blob_to_vec(&output) })
    }
}

/// 把 32 字节密钥编码为落盘形态。
#[cfg(windows)]
fn encode_key_at_rest(key: &[u8; KEY_LEN]) -> std::io::Result<Vec<u8>> {
    let wrapped = at_rest::protect(key)?;
    let mut payload = Vec::with_capacity(KEYSTORE_MAGIC.len() + wrapped.len());
    payload.extend_from_slice(KEYSTORE_MAGIC);
    payload.extend_from_slice(&wrapped);
    Ok(payload)
}

#[cfg(not(windows))]
fn encode_key_at_rest(key: &[u8; KEY_LEN]) -> std::io::Result<Vec<u8>> {
    Ok(key.to_vec())
}

/// 读取或创建本地密钥。
///
/// - Windows：文件为 `ACEKST1` 魔数 + DPAPI 密文（绑定当前 Windows 用户）；
///   历史裸 32 字节格式读取成功后就地升级为封装格式。
/// - Unix：裸 32 字节 + `0600` 权限。
///
/// 文件存在但内容无效时返回错误，**不静默重建**（静默轮换密钥等于放弃
/// 既有密文的可读性）。读取失败（DPAPI 解封失败等）同样上抛，由调用方
/// 决定降级策略——本项目对敏感项的策略是拒绝读写而非明文。
pub fn load_or_create_key(dir: &std::path::Path) -> std::io::Result<[u8; KEY_LEN]> {
    let path = dir.join(KEYSTORE_FILE);

    match std::fs::read(&path) {
        Ok(bytes) => {
            #[cfg(windows)]
            {
                if bytes.starts_with(KEYSTORE_MAGIC) {
                    let plain = at_rest::unprotect(&bytes[KEYSTORE_MAGIC.len()..])?;
                    if plain.len() != KEY_LEN {
                        return Err(std::io::Error::other(
                            "keystore.key 内容无效（DPAPI 解封后长度错误）",
                        ));
                    }
                    let mut key = [0u8; KEY_LEN];
                    key.copy_from_slice(&plain);
                    return Ok(key);
                }
            }
            if bytes.len() == KEY_LEN {
                let mut key = [0u8; KEY_LEN];
                key.copy_from_slice(&bytes);
                // 历史裸格式：Windows 下就地升级为 DPAPI 封装（失败保留原文件，下次再试）
                #[cfg(windows)]
                {
                    match encode_key_at_rest(&key) {
                        Ok(payload) => {
                            if std::fs::write(&path, payload).is_ok() {
                                tracing::info!("keystore.key 已升级为 DPAPI 封装格式");
                            }
                        }
                        Err(e) => tracing::warn!("keystore.key DPAPI 升级失败（保留原格式）: {}", e),
                    }
                }
                return Ok(key);
            }
            Err(std::io::Error::other("keystore.key 内容无效（长度不匹配）"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut key = [0u8; KEY_LEN];
            OsRng.fill_bytes(&mut key);

            std::fs::create_dir_all(dir)?;
            std::fs::write(&path, encode_key_at_rest(&key)?)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }

            Ok(key)
        }
        Err(e) => Err(e),
    }
}

/// 使用本地密钥加密，返回带 `enc:v1:` 前缀的密文
pub fn encrypt_secret(plaintext: &str, key: &[u8; KEY_LEN]) -> AppResult<String> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("创建cipher失败: {}", e)))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Crypto(format!("加密失败: {}", e)))?;

    let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    Ok(format!(
        "{}{}",
        SECRET_PREFIX,
        general_purpose::STANDARD.encode(&payload)
    ))
}

/// 是否为本地密钥加密后的密文
pub fn is_encrypted_secret(data: &str) -> bool {
    data.starts_with(SECRET_PREFIX)
}

/// 使用本地密钥解密；格式不符或密钥不匹配时返回 None
/// （例如数据库从其它机器导入，密钥文件不同）
pub fn decrypt_secret(encrypted: &str, key: &[u8; KEY_LEN]) -> Option<String> {
    let body = encrypted.strip_prefix(SECRET_PREFIX)?;
    let data = general_purpose::STANDARD.decode(body).ok()?;

    if data.len() < NONCE_LEN {
        return None;
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).ok()?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;

    String::from_utf8(plaintext).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let api_key = "sk-ant-1234567890";
        let password = "my-secret-password";

        let encrypted = encrypt_api_key(api_key, password).unwrap();
        let decrypted = decrypt_api_key(&encrypted, password).unwrap();

        assert_eq!(api_key, decrypted);
    }

    #[test]
    fn test_wrong_password_fails() {
        let api_key = "sk-ant-1234567890";
        let password = "my-secret-password";
        let wrong_password = "wrong-password";

        let encrypted = encrypt_api_key(api_key, password).unwrap();
        let result = decrypt_api_key(&encrypted, wrong_password);

        assert!(result.is_err());
    }

    #[test]
    fn local_key_roundtrip_and_prefix() {
        let key = [7u8; KEY_LEN];
        let sealed = encrypt_secret("sk-ant-secret", &key).unwrap();

        assert!(is_encrypted_secret(&sealed));
        assert_ne!(sealed, "sk-ant-secret");
        assert_eq!(decrypt_secret(&sealed, &key).unwrap(), "sk-ant-secret");
    }

    #[test]
    fn local_key_mismatch_returns_none() {
        let sealed = encrypt_secret("sk-ant-secret", &[1u8; KEY_LEN]).unwrap();
        assert!(decrypt_secret(&sealed, &[2u8; KEY_LEN]).is_none());
    }

    #[test]
    fn plaintext_is_not_treated_as_secret() {
        assert!(!is_encrypted_secret("sk-ant-plain"));
        assert!(decrypt_secret("sk-ant-plain", &[3u8; KEY_LEN]).is_none());
    }

    #[test]
    fn keystore_is_created_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("ace-keystore-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        let first = load_or_create_key(&dir).unwrap();
        let second = load_or_create_key(&dir).unwrap();
        assert_eq!(first, second, "重复调用应复用同一密钥");

        // 用第一次的密钥加密，第二次读取的密钥应能解密
        let sealed = encrypt_secret("value", &first).unwrap();
        assert_eq!(decrypt_secret(&sealed, &second).unwrap(), "value");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
