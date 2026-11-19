//! Putting files on the system clipboard, as a file manager does, so that
//! Ctrl+V in a folder of Windows Explorer copies them there.
//!
//! Windows keeps such a list as `CF_HDROP`: a `DROPFILES` header and then
//! the paths, each ended by a nul, and one more nul after the last.

use std::path::PathBuf;

/// The size of the `DROPFILES` header: the offset of the names, a point,
/// and two flags, each four bytes.
const HEADER: usize = 20;

/// The bytes of the `CF_HDROP` data for `paths`: names written with
/// backslashes, in UTF-16.
pub fn drop_files(paths: &[PathBuf]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER + paths.len() * 64);
    let header: [u32; 5] = [HEADER as u32, 0, 0, 0, 1]; // names at 20, point 0,0, not in the non-client area, wide
    for word in header {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for path in paths {
        let name = path.to_string_lossy().replace('/', "\\");
        // The `\\?\` form some APIs give is not one Explorer takes in a list.
        let name = name.strip_prefix(r"\\?\").unwrap_or(&name);
        for unit in name.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

/// Puts `paths` on the clipboard as files to copy.
#[cfg(windows)]
pub fn copy_files(paths: &[PathBuf]) -> Result<(), String> {
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

    /// The clipboard format of a list of files.
    const CF_HDROP: u32 = 15;
    /// What a paste does with them: copy.
    const DROPEFFECT_COPY: u32 = 1;

    /// A block of memory for the clipboard, filled with `bytes`.
    unsafe fn block(bytes: &[u8]) -> Result<*mut core::ffi::c_void, String> {
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if h.is_null() {
            return Err("out of memory".into());
        }
        let at = GlobalLock(h) as *mut u8;
        if at.is_null() {
            GlobalFree(h);
            return Err("cannot lock memory".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), at, bytes.len());
        GlobalUnlock(h);
        Ok(h)
    }

    // SAFETY: plain calls of the clipboard functions with memory this
    // function allocates; the clipboard takes the blocks it is given, and
    // a block it does not take is freed here.
    unsafe {
        // Another program may hold the clipboard for a moment.
        let mut open = false;
        for _ in 0..10 {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                open = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !open {
            return Err("the clipboard is in use".into());
        }
        EmptyClipboard();
        let result = (|| {
            let files = block(&drop_files(paths))?;
            if SetClipboardData(CF_HDROP, files).is_null() {
                GlobalFree(files);
                return Err("the clipboard did not take the files".to_string());
            }
            // Which way a paste goes: copying. Without it Explorer may choose.
            let name: Vec<u16> = "Preferred DropEffect".encode_utf16().chain([0]).collect();
            let format = RegisterClipboardFormatW(name.as_ptr());
            if format != 0 {
                let effect = block(&DROPEFFECT_COPY.to_le_bytes())?;
                if SetClipboardData(format, effect).is_null() {
                    GlobalFree(effect);
                }
            }
            Ok(())
        })();
        CloseClipboard();
        result
    }
}

/// Only Windows has a clipboard of files here.
#[cfg(not(windows))]
pub fn copy_files(_: &[PathBuf]) -> Result<(), String> {
    Err("copying files to the clipboard is for Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(bytes: &[u8]) -> Vec<u16> {
        bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
    }

    #[test]
    fn the_list_is_a_dropfiles_header_and_nul_ended_wide_names() {
        let bytes = drop_files(&[PathBuf::from("E:/Music/a.mp3"), PathBuf::from(r"E:\Music\Å ö\b.flac")]);
        // pFiles = 20, the point, fNC = 0, fWide = 1.
        let header: Vec<u32> = bytes[..20].chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        assert_eq!(header, [20, 0, 0, 0, 1]);
        let names = String::from_utf16(&words(&bytes[20..])).unwrap();
        assert_eq!(names, "E:\\Music\\a.mp3\0E:\\Music\\Å ö\\b.flac\0\0", "backslashes, a nul after each and one more");
        assert_eq!(bytes.len() % 2, 0);
    }

    #[test]
    fn the_long_path_prefix_is_left_off() {
        let bytes = drop_files(&[PathBuf::from(r"\\?\E:\Music\a.mp3")]);
        assert_eq!(String::from_utf16(&words(&bytes[20..])).unwrap(), "E:\\Music\\a.mp3\0\0");
    }

    #[test]
    fn the_playlist_menu_can_copy_the_playlist() {
        let m = crate::model::Model::for_tests();
        let menu = m.playlist_row_menu();
        assert!(menu.iter().any(|i| format!("{i:?}").contains("Copy to clipboard")));
    }

    /// Puts real files on the real clipboard and reads the list back, as
    /// Explorer's paste does. It replaces what the clipboard held, so it
    /// runs on request: `cargo test -p popamp clipboard -- --ignored`.
    #[cfg(windows)]
    #[test]
    #[ignore = "replaces the contents of the clipboard"]
    fn files_on_the_clipboard_can_be_read_back_as_a_file_list() {
        use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
        use windows_sys::Win32::UI::Shell::DragQueryFileW;
        let dir = std::env::temp_dir().join(format!("popamp-clipboard-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let files: Vec<PathBuf> = ["one.mp3", "two \u{e5}.flac"].iter().map(|n| dir.join(n)).collect();
        for f in &files {
            std::fs::write(f, b"x").unwrap();
        }
        copy_files(&files).unwrap();
        let mut read = Vec::new();
        unsafe {
            assert_ne!(OpenClipboard(std::ptr::null_mut()), 0);
            let drop = GetClipboardData(15);
            assert!(!drop.is_null(), "a file list is on the clipboard");
            let n = DragQueryFileW(drop, u32::MAX, std::ptr::null_mut(), 0);
            for i in 0..n {
                let mut buf = [0u16; 520];
                let len = DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32) as usize;
                read.push(String::from_utf16(&buf[..len]).unwrap());
            }
            CloseClipboard();
        }
        let wanted: Vec<String> = files.iter().map(|f| f.to_string_lossy().replace('/', "\\")).collect();
        assert_eq!(read, wanted);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn no_files_is_an_empty_list_with_its_closing_nul() {
        let bytes = drop_files(&[]);
        assert_eq!(bytes.len(), 22);
        assert_eq!(&bytes[20..], [0, 0]);
    }
}
