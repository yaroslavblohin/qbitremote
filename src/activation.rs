//! File validation shared by startup and native shell activation.
use crate::add_torrent::Source;
use std::{ffi::OsString, io::Read, path::PathBuf};

pub fn requests(args: impl IntoIterator<Item = OsString>) -> Result<Vec<Source>, String> {
    let mut requests = Vec::new();
    for arg in args {
        let source = if let Some(source) = arg.to_str().and_then(magnet_link) {
            source
        } else {
            file_request(arg.into())?
        };
        if !requests.contains(&source) {
            requests.push(source);
        }
    }
    Ok(requests)
}

pub fn file_request(path: PathBuf) -> Result<Source, String> {
    let extension = path.extension().unwrap_or_default();
    let is_magnet = extension.eq_ignore_ascii_case("magnet");
    if !is_magnet && !extension.eq_ignore_ascii_case("torrent") {
        return Err(format!(
            "Поддерживаются файлы .torrent и .magnet: {}",
            path.display()
        ));
    }
    let path = std::fs::canonicalize(&path)
        .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))?;
    if !path.is_file() {
        return Err(format!("Это не файл: {}", path.display()));
    }
    if is_magnet {
        read_magnet_file(&path)
    } else {
        Ok(Source::File(path))
    }
}

pub fn magnet_link(value: &str) -> Option<Source> {
    let url = url::Url::parse(value)
        .ok()
        .filter(|url| url.scheme() == "magnet")?;
    Some(Source::Link {
        url: url.into(),
        downloader: String::new(),
    })
}

fn read_magnet_file(path: &std::path::Path) -> Result<Source, String> {
    const LIMIT: usize = 64 * 1024;
    let mut data = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take((LIMIT + 1) as u64).read_to_end(&mut data))
        .map_err(|e| format!("Не удалось прочитать {}: {e}", path.display()))?;
    let invalid = || format!("Файл {} должен содержать magnet-ссылку", path.display());
    if data.len() > LIMIT {
        return Err(invalid());
    }
    let text = std::str::from_utf8(&data).map_err(|_| invalid())?;
    let text = text.trim_start_matches('\u{feff}').trim();
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().trim();
    let candidate = if first.eq_ignore_ascii_case("[InternetShortcut]") {
        lines.find_map(|line| {
            let (key, value) = line.split_once('=')?;
            key.trim().eq_ignore_ascii_case("URL").then(|| value.trim())
        })
    } else {
        lines.next().is_none().then_some(first)
    };
    candidate.and_then(magnet_link).ok_or_else(invalid)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        os::windows::ffi::{OsStrExt, OsStringExt},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{DataExchange::COPYDATASTRUCT, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::*,
    };
    const MAGIC: usize = 0x51425231;
    const LIMIT: usize = 1024 * 1024;
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn name() -> &'static str {
        if cfg!(feature = "ui-smoke") {
            "qBitRemote.Activation.Smoke.v1"
        } else {
            "qBitRemote.Activation.v1"
        }
    }
    struct State {
        tx: mpsc::Sender<Vec<OsString>>,
        ctx: eframe::egui::Context,
    }
    unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        match msg {
            WM_COPYDATA => {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
                if state.is_null() || l == 0 {
                    return 0;
                }
                let data = &*(l as *const COPYDATASTRUCT);
                if data.dwData != MAGIC
                    || data.cbData == 0
                    || data.cbData as usize > LIMIT
                    || data.lpData.is_null()
                {
                    return 0;
                }
                let bytes =
                    std::slice::from_raw_parts(data.lpData as *const u8, data.cbData as usize);
                let Ok(args) = serde_json::from_slice::<Vec<Vec<u16>>>(bytes) else {
                    return 0;
                };
                let args = args.iter().map(|s| OsString::from_wide(s)).collect();
                if (*state).tx.send(args).is_err() {
                    return 0;
                }
                (*state).ctx.request_repaint();
                1
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
    pub struct Listener {
        pub rx: Receiver<Vec<OsString>>,
        hwnd: isize,
        thread: Option<JoinHandle<()>>,
    }
    impl Listener {
        pub fn start(ctx: eframe::egui::Context) -> Result<Self, String> {
            Self::named(name(), ctx)
        }
        fn named(title: &str, ctx: eframe::egui::Context) -> Result<Self, String> {
            let title = title.to_owned();
            let (tx, rx) = mpsc::channel();
            let (ready, out) = mpsc::sync_channel(1);
            let thread = thread::spawn(move || unsafe {
                let class = wide("qBitRemote.Activation.Window.v1");
                let title = wide(&title);
                let instance = GetModuleHandleW(std::ptr::null());
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: instance,
                    lpszClassName: class.as_ptr(),
                    ..std::mem::zeroed()
                };
                RegisterClassW(&wc);
                let hwnd = CreateWindowExW(
                    0,
                    class.as_ptr(),
                    title.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                if hwnd.is_null() {
                    let _ = ready.send(Err(std::io::Error::last_os_error().to_string()));
                    return;
                }
                let state = Box::new(State { tx, ctx });
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*state as *const State) as isize);
                let _ = ready.send(Ok(hwnd as isize));
                let mut message = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                drop(state);
            });
            let hwnd = out.recv().map_err(|e| e.to_string())??;
            Ok(Self {
                rx,
                hwnd,
                thread: Some(thread),
            })
        }
    }
    impl Drop for Listener {
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.hwnd as _, WM_CLOSE, 0, 0);
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
    pub fn forward(args: &[OsString]) -> Result<(), String> {
        forward_named(name(), args)
    }
    fn forward_named(title: &str, args: &[OsString]) -> Result<(), String> {
        let data: Vec<Vec<u16>> = args.iter().map(|s| s.encode_wide().collect()).collect();
        let bytes = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
        if bytes.len() > LIMIT {
            return Err("Слишком много файлов в одном запросе".into());
        }
        let title = wide(title);
        let class = wide("qBitRemote.Activation.Window.v1");
        let deadline = Instant::now() + Duration::from_secs(8);
        let hwnd = loop {
            let h = unsafe { FindWindowW(class.as_ptr(), title.as_ptr()) };
            if !h.is_null() {
                break h;
            }
            if Instant::now() >= deadline {
                return Err("Работающий клиент не принимает файлы. Завершите его через меню «Выход» и запустите новую версию.".into());
            }
            thread::sleep(Duration::from_millis(50));
        };
        let data = COPYDATASTRUCT {
            dwData: MAGIC,
            cbData: bytes.len() as u32,
            lpData: bytes.as_ptr() as _,
        };
        let mut result = 0;
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd,
                WM_COPYDATA,
                0,
                (&data as *const COPYDATASTRUCT) as isize,
                SMTO_ABORTIFHUNG,
                3000,
                &mut result,
            )
        };
        if sent == 0 || result != 1 {
            return Err("Не удалось передать файл работающему клиенту. Повторите открытие после восстановления окна.".into());
        }
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn forwards_from_another_process() {
            use std::os::windows::process::CommandExt;
            let title = format!("qBitRemote.Test.Activation.Process.{}", std::process::id());
            let listener = Listener::named(&title, Default::default()).unwrap();
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "activation::windows::tests::child_forward",
                    "--ignored",
                ])
                .env("QBIT_ACTIVATION_TEST_TITLE", &title)
                .creation_flags(0x08000000)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
            assert_eq!(
                listener.rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                vec![OsString::from("C:\\Папка с пробелами\\торрент.torrent")]
            );
        }
        #[test]
        #[ignore = "launched by cross-process activation test"]
        fn child_forward() {
            forward_named(
                &std::env::var("QBIT_ACTIVATION_TEST_TITLE").unwrap(),
                &[OsString::from("C:\\Папка с пробелами\\торрент.torrent")],
            )
            .unwrap();
        }
        #[test]
        fn activation_delivers_paths_and_empty_restore_request() {
            let title = format!("qBitRemote.Test.Activation.{}", std::process::id());
            let listener = Listener::named(&title, Default::default()).unwrap();
            let args = vec![
                OsString::from("C:\\Тест папка\\фильм.torrent"),
                OsString::from("D:\\other.torrent"),
            ];
            forward_named(&title, &args).unwrap();
            assert_eq!(
                listener.rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                args
            );
            forward_named(&title, &[]).unwrap();
            assert!(listener
                .rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .is_empty());
        }
    }
}
#[cfg(windows)]
pub use windows::{forward, Listener};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_paths_and_deduplicates() {
        let path =
            std::env::temp_dir().join(format!("qbit-открыть {}.TORRENT", std::process::id()));
        std::fs::write(&path, b"test").unwrap();
        assert_eq!(
            requests([path.clone().into_os_string(), path.clone().into_os_string()])
                .unwrap()
                .len(),
            1
        );
        std::fs::remove_file(&path).unwrap();
        assert!(requests([path.into_os_string()]).is_err());
        assert!(requests([OsString::from("wrong.txt")]).is_err());
    }

    #[test]
    fn opens_magnet_links_and_shortcuts() {
        let path = std::env::temp_dir().join(format!("qbit ссылка {}.MAGNET", std::process::id()));
        let link = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567";
        for content in [
            format!("\u{feff}{link}\n"),
            format!("[InternetShortcut]\r\nURL={link}\r\n"),
        ] {
            std::fs::write(&path, content).unwrap();
            assert_eq!(
                requests([path.clone().into_os_string(), OsString::from(link)]).unwrap(),
                [magnet_link(link).unwrap()]
            );
        }
        for content in ["https://example.com".to_owned(), "x".repeat(64 * 1024 + 1)] {
            std::fs::write(&path, content).unwrap();
            assert!(file_request(path.clone()).is_err());
        }
        std::fs::remove_file(path).unwrap();
    }
}
