use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/winscroller2d.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ico_path = manifest_dir.join("assets").join("winscroller2d.ico");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    match env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => {
            let ico_bytes = match fs::read(&ico_path) {
                Ok(b) => b,
                Err(err) => {
                    println!("cargo:warning=Could not read {}: {err}", ico_path.display());
                    return;
                }
            };
            let Some(res_bytes) = build_icon_res(&ico_bytes) else {
                println!(
                    "cargo:warning=Invalid ICO file at {}; skipping icon embed",
                    ico_path.display()
                );
                return;
            };
            let res_path = out_dir.join("winscroller2d.res");
            fs::write(&res_path, res_bytes).expect("write winscroller2d.res");
            println!(
                "cargo:rustc-link-arg-bin=winscroller2d={}",
                res_path.display()
            );
        }
        Ok("gnu") => {
            let rc_path = out_dir.join("winscroller2d.rc");
            let obj_path = out_dir.join("winscroller2d_res.o");
            let manifest_path = out_dir.join("winscroller2d.manifest");
            let manifest_escaped = manifest_path.to_string_lossy().replace('\\', "/");
            fs::write(&manifest_path, APP_MANIFEST).expect("write manifest");

            let escaped = ico_path.to_string_lossy().replace('\\', "/");
            let rc_text = format!("1 ICON \"{escaped}\"\n1 24 \"{manifest_escaped}\"\n");
            fs::write(&rc_path, rc_text).expect("write winscroller2d.rc");

            let status = Command::new("windres")
                .args([
                    "-i",
                    &rc_path.to_string_lossy(),
                    "-O",
                    "coff",
                    "-o",
                    &obj_path.to_string_lossy(),
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!(
                        "cargo:rustc-link-arg-bin=winscroller2d={}",
                        obj_path.display()
                    );
                }
                Ok(s) => {
                    println!("cargo:warning=windres exited with {s}; skipping icon embed");
                }
                Err(err) => {
                    println!("cargo:warning=windres not available ({err}); skipping icon embed");
                }
            }
        }
        _ => {}
    }
}

fn push_res_entry(out: &mut Vec<u8>, res_type: u16, res_id: u16, data: &[u8]) {
    let header_size: u32 = 32;
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_type.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0x1030u16.to_le_bytes()); // MOVEABLE | PURE | DISCARDABLE
    out.extend_from_slice(&0x0409u16.to_le_bytes()); // en-US
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// Converts a standard Windows `.ico` buffer into a compiled 32-bit `.res` resource file
/// with `RT_GROUP_ICON` ID `1` and `RT_ICON` IDs `1..=N`.
fn build_icon_res(ico: &[u8]) -> Option<Vec<u8>> {
    if ico.len() < 6 {
        return None;
    }
    let reserved = u16::from_le_bytes([ico[0], ico[1]]);
    let ico_type = u16::from_le_bytes([ico[2], ico[3]]);
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    if reserved != 0 || ico_type != 1 || count == 0 || ico.len() < 6 + count * 16 {
        return None;
    }

    let mut res = Vec::with_capacity(ico.len() + 256);
    res.extend_from_slice(&0u32.to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes());
    res.extend_from_slice(&0xFFFFu16.to_le_bytes());
    res.extend_from_slice(&0u16.to_le_bytes());
    res.extend_from_slice(&0xFFFFu16.to_le_bytes());
    res.extend_from_slice(&0u16.to_le_bytes());
    res.extend_from_slice(&[0u8; 16]);

    let mut group = Vec::with_capacity(6 + count * 14);
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(count as u16).to_le_bytes());

    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;

    for i in 0..count {
        let dir = &ico[6 + i * 16..6 + (i + 1) * 16];
        let bytes_in_res = u32::from_le_bytes([dir[8], dir[9], dir[10], dir[11]]) as usize;
        let image_offset = u32::from_le_bytes([dir[12], dir[13], dir[14], dir[15]]) as usize;
        let end = image_offset.checked_add(bytes_in_res)?;
        if end > ico.len() {
            return None;
        }
        let icon_id = (i + 1) as u16;
        push_res_entry(&mut res, RT_ICON, icon_id, &ico[image_offset..end]);
        group.extend_from_slice(&dir[0..12]);
        group.extend_from_slice(&icon_id.to_le_bytes());
    }

    push_res_entry(&mut res, RT_GROUP_ICON, 1, &group);

    const RT_MANIFEST: u16 = 24;
    push_res_entry(&mut res, RT_MANIFEST, 1, APP_MANIFEST.as_bytes());

    Some(res)
}

const APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2,PerMonitor</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:asm.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#;
