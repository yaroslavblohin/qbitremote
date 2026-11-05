//! Passwords live in the user's login Keychain, not the settings file.
use crate::model::Settings;
use std::ffi::c_void;
#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecKeychainFindGenericPassword(
        keychain: *const c_void,
        service_len: u32,
        service: *const u8,
        account_len: u32,
        account: *const u8,
        length: *mut u32,
        data: *mut *mut c_void,
        item: *mut *mut c_void,
    ) -> i32;
    fn SecKeychainAddGenericPassword(
        keychain: *const c_void,
        service_len: u32,
        service: *const u8,
        account_len: u32,
        account: *const u8,
        length: u32,
        data: *const u8,
        item: *mut *mut c_void,
    ) -> i32;
    fn SecKeychainItemModifyAttributesAndData(
        item: *const c_void,
        attrs: *const c_void,
        length: u32,
        data: *const u8,
    ) -> i32;
    fn SecKeychainItemDelete(item: *const c_void) -> i32;
    fn SecKeychainItemFreeContent(attrs: *const c_void, data: *mut c_void) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: *const c_void);
}
const SERVICE: &[u8] = b"org.qbitremote.password";
fn account(settings: &Settings) -> Result<String, String> {
    let mut url = url::Url::parse(settings.address.trim()).map_err(|e| e.to_string())?;
    url.set_path(&format!("{}/", url.path().trim_end_matches('/')));
    Ok(format!("{}\n{}", url, settings.username))
}
fn check(status: i32) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("Keychain: ошибка {status}"))
    }
}
pub fn load(settings: &Settings) -> Result<String, String> {
    if !settings.remember_password {
        return Ok(String::new());
    }
    let account = account(settings)?;
    let mut length = 0;
    let mut data = std::ptr::null_mut();
    unsafe {
        let status = SecKeychainFindGenericPassword(
            std::ptr::null(),
            SERVICE.len() as u32,
            SERVICE.as_ptr(),
            account.len() as u32,
            account.as_ptr(),
            &mut length,
            &mut data,
            std::ptr::null_mut(),
        );
        if status == -25300 {
            return Ok(String::new());
        }
        check(status)?;
        let result = if length == 0 {
            Ok(String::new())
        } else {
            String::from_utf8(
                std::slice::from_raw_parts(data.cast::<u8>(), length as usize).to_vec(),
            )
            .map_err(|e| e.to_string())
        };
        SecKeychainItemFreeContent(std::ptr::null(), data);
        result
    }
}
pub fn store(settings: &mut Settings, password: &str) -> Result<(), String> {
    let account = account(settings)?;
    let mut item = std::ptr::null_mut();
    unsafe {
        let status = SecKeychainFindGenericPassword(
            std::ptr::null(),
            SERVICE.len() as u32,
            SERVICE.as_ptr(),
            account.len() as u32,
            account.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut item,
        );
        if status != 0 && status != -25300 {
            return check(status);
        }
        let result = if status == 0 {
            let result = if settings.remember_password {
                SecKeychainItemModifyAttributesAndData(
                    item,
                    std::ptr::null(),
                    password.len() as u32,
                    password.as_ptr(),
                )
            } else {
                SecKeychainItemDelete(item)
            };
            CFRelease(item);
            result
        } else if settings.remember_password {
            SecKeychainAddGenericPassword(
                std::ptr::null(),
                SERVICE.len() as u32,
                SERVICE.as_ptr(),
                account.len() as u32,
                account.as_ptr(),
                password.len() as u32,
                password.as_ptr(),
                std::ptr::null_mut(),
            )
        } else {
            0
        };
        check(result)?;
    }
    settings.saved_password = None;
    Ok(())
}
