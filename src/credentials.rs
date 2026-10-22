//! Passwords are protected for the current Windows user, never in plaintext.
use crate::model::{SavedPassword, Settings};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

fn identity(settings: &Settings) -> Result<String, String> {
    let mut address = url::Url::parse(settings.address.trim())
        .map_err(|_| "Некорректный адрес сервера".to_string())?;
    address.set_path(&format!("{}/", address.path().trim_end_matches('/')));
    Ok(format!("qBit Remote\n{}\n{}", address, settings.username))
}

pub fn load(settings: &Settings) -> Result<String, String> {
    if !settings.remember_password {
        return Ok(String::new());
    }
    let Some(saved) = &settings.saved_password else {
        return Ok(String::new());
    };
    let identity = identity(settings)?;
    if saved.identity != identity {
        return Ok(String::new());
    }
    String::from_utf8(transform(&saved.encrypted, identity.as_bytes(), false)?)
        .map_err(|_| "Не удалось прочитать сохранённый пароль".into())
}

pub fn store(settings: &mut Settings, password: &str) -> Result<(), String> {
    if !settings.remember_password {
        settings.saved_password = None;
        return Ok(());
    }
    let identity = identity(settings)?;
    let encrypted = transform(password.as_bytes(), identity.as_bytes(), true)?;
    settings.saved_password = Some(SavedPassword {
        identity,
        encrypted,
    });
    Ok(())
}

fn transform(data: &[u8], entropy: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data
            .len()
            .try_into()
            .map_err(|_| "Пароль слишком длинный")?,
        pbData: data.as_ptr().cast_mut(),
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: entropy
            .len()
            .try_into()
            .map_err(|_| "Адрес слишком длинный")?,
        pbData: entropy.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI reads our buffers and allocates its output with LocalAlloc.
    // No machine-wide flag: decryption is scoped to this Windows account.
    unsafe {
        let ok = if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(format!(
                "Хранилище пароля Windows: {}",
                std::io::Error::last_os_error()
            ));
        }
        let result = if output.cbData == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
        };
        // Clear the decrypted native allocation before releasing it.
        for i in 0..output.cbData as usize {
            std::ptr::write_volatile(output.pbData.add(i), 0);
        }
        LocalFree(output.pbData.cast());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_preserve_separate_passwords_across_restart() {
        let mut settings = Settings::default();
        store(&mut settings, "first-password").unwrap();
        settings.save_server();
        settings.selected_server = 1;
        settings.address = "http://second-host:8080".into();
        settings.server_name = "Second".into();
        store(&mut settings, "second-password").unwrap();
        settings.save_server();
        let serialized = ron::to_string(&settings).unwrap();
        assert!(!serialized.contains("first-password"));
        assert!(!serialized.contains("second-password"));
        let mut restored: Settings = ron::from_str(&serialized).unwrap();
        restored.select_server(restored.selected_server);
        assert_eq!(load(&restored).unwrap(), "second-password");
        restored.select_server(0);
        assert_eq!(load(&restored).unwrap(), "first-password");
        restored.remember_password = false;
        store(&mut restored, "first-password").unwrap();
        restored.save_server();
        restored.select_server(1);
        assert_eq!(load(&restored).unwrap(), "second-password");
        restored.select_server(0);
        assert_eq!(load(&restored).unwrap(), "");
    }
    #[test]
    fn survives_settings_serialization_without_plaintext() {
        let mut settings = Settings::default();
        let password = "test-пароль-&-🔒";
        store(&mut settings, password).unwrap();
        let serialized = ron::to_string(&settings).unwrap();
        assert!(!serialized.contains(password));
        let restored: Settings = ron::from_str(&serialized).unwrap();
        assert_eq!(load(&restored).unwrap(), password);
    }
    #[test]
    fn never_reuses_password_for_another_server_or_user() {
        let mut settings = Settings::default();
        store(&mut settings, "secret").unwrap();
        settings.username = "other".into();
        assert_eq!(load(&settings).unwrap(), "");
        settings.username = "admin".into();
        settings.address = "http://another-host:8080".into();
        assert_eq!(load(&settings).unwrap(), "");
    }
    #[test]
    fn equivalent_url_and_forget_password() {
        let mut settings = Settings::default();
        store(&mut settings, "secret").unwrap();
        settings.address.push('/');
        assert_eq!(load(&settings).unwrap(), "secret");
        settings.remember_password = false;
        store(&mut settings, "secret").unwrap();
        assert!(settings.saved_password.is_none());
        assert_eq!(load(&settings).unwrap(), "");
    }
    #[test]
    fn tampering_is_reported() {
        let mut settings = Settings::default();
        store(&mut settings, "secret").unwrap();
        settings.saved_password.as_mut().unwrap().encrypted[0] ^= 1;
        assert!(load(&settings).is_err());
    }
    #[test]
    fn old_settings_still_load() {
        let settings: Settings = ron::from_str(
            "(address:\"http://host:8080\",username:\"admin\",close_to_tray:true,zoom:1.0)",
        )
        .unwrap();
        assert!(settings.remember_password);
        assert_eq!(load(&settings).unwrap(), "");
    }
}
