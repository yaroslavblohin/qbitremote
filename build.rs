#[path = "src/icon_pixels.rs"]
mod icon_pixels;

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/icon_pixels.rs");
    println!("cargo:rerun-if-changed=assets/logo.rgba");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=WindowsSdkDir");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("app.ico"), icon()).unwrap();
    fs::write(out.join("app.rc"), "1 ICON \"app.ico\"\n").unwrap();
    let compiler = resource_compiler();
    let output = Command::new(&compiler)
        .current_dir(&out)
        .args(["/nologo", "/fo", "app.res", "app.rc"])
        .output()
        .expect("Cannot start Windows resource compiler (rc.exe)");
    assert!(
        output.status.success(),
        "Resource compilation failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!(
        "cargo:rustc-link-arg-bin=qbit-remote={}",
        out.join("app.res").display()
    );
}

fn resource_compiler() -> PathBuf {
    if let Some(rc) = env::var_os("RC") {
        return rc.into();
    }
    let sdk = env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                env::var_os("ProgramFiles(x86)").expect("Windows SDK required; set RC to rc.exe"),
            )
            .join("Windows Kits/10")
        });
    let bin = sdk.join("bin");
    let mut versions: Vec<_> = fs::read_dir(&bin)
        .expect("Windows SDK bin directory missing")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("10."))
        .collect();
    versions.sort();
    let host = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    for version in versions.into_iter().rev().chain(std::iter::once(bin)) {
        let rc = version.join(host).join("rc.exe");
        if rc.is_file() {
            return rc;
        }
    }
    panic!("rc.exe not found; install Windows SDK or set RC");
}

// ICO images use bottom-up BGRA DIBs and a padded 1-bit transparency mask.
fn icon() -> Vec<u8> {
    let sizes = [16_u32, 24, 32, 48, 64, 128, 256];
    let mut result = vec![0, 0, 1, 0];
    result.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + sizes.len() * 16;
    let mut images = Vec::new();
    for size in sizes {
        let pixels = icon_pixels::rgba(size);
        let stride = size.div_ceil(32) as usize * 4;
        let mut dib = Vec::new();
        for value in [40, size, size * 2] {
            dib.extend_from_slice(&value.to_le_bytes());
        }
        dib.extend_from_slice(&1_u16.to_le_bytes());
        dib.extend_from_slice(&32_u16.to_le_bytes());
        for value in [0_u32, size * size * 4, 0, 0, 0, 0] {
            dib.extend_from_slice(&value.to_le_bytes());
        }
        let mut mask = vec![0_u8; stride * size as usize];
        for y in (0..size as usize).rev() {
            for x in 0..size as usize {
                let i = (y * size as usize + x) * 4;
                dib.extend_from_slice(&[pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3]]);
                if pixels[i + 3] == 0 {
                    mask[(size as usize - 1 - y) * stride + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        dib.extend_from_slice(&mask);
        result.extend_from_slice(&[size as u8, size as u8, 0, 0]);
        result.extend_from_slice(&1_u16.to_le_bytes());
        result.extend_from_slice(&32_u16.to_le_bytes());
        result.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        result.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += dib.len();
        images.extend_from_slice(&dib);
    }
    result.extend_from_slice(&images);
    result
}
