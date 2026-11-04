use super::*;
use std::{ffi::OsStr, fs::Metadata};

#[cfg(unix)]
pub(super) fn mode(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(windows)]
pub(super) fn mode(metadata: &Metadata) -> u32 {
    let kind = if metadata.is_dir() {
        0o040111
    } else {
        0o100000
    };
    kind | if metadata.permissions().readonly() {
        0o444
    } else {
        0o644
    }
}

#[cfg(unix)]
pub(super) fn name_bytes(name: &OsStr) -> FsResult<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Ok(name.as_bytes().to_vec())
}

#[cfg(windows)]
pub(super) fn name_bytes(name: &OsStr) -> FsResult<Vec<u8>> {
    // Guest paths are UTF-8. Reject unpaired host surrogates, not lossy aliases.
    Ok(name.to_str().ok_or(Error(22))?.as_bytes().to_vec())
}

pub(super) fn link_bytes(path: &Path) -> FsResult<Vec<u8>> {
    #[cfg(windows)]
    {
        // A Windows drive/UNC target cannot denote an Android rootfs path.
        if path
            .components()
            .any(|part| matches!(part, std::path::Component::Prefix(_)))
        {
            return Err(Error(22));
        }
        Ok(name_bytes(path.as_os_str())?
            .into_iter()
            .map(|b| if b == b'\\' { b'/' } else { b })
            .collect())
    }
    #[cfg(unix)]
    name_bytes(path.as_os_str())
}

pub(super) fn validate_component(part: &str) -> FsResult<()> {
    // Android separators are '/'. Never let a host separator, drive, or ADS
    // syntax reinterpret a single guest component and escape the rootfs.
    #[cfg(windows)]
    if part.contains(['\\', ':']) || part.ends_with(['.', ' ']) {
        return Err(Error(22));
    }
    #[cfg(windows)]
    {
        // Win32 also interprets device names inside otherwise ordinary paths.
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|n| {
                    matches!(
                        n,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            })
        {
            return Err(Error(22));
        }
    }
    let _ = part;
    Ok(())
}

#[cfg(windows)]
pub(super) fn stat(metadata: &Metadata, path: &str) -> crate::arch::Stat {
    use std::os::windows::fs::MetadataExt;
    fn unix_time(filetime: u64) -> (i64, i64) {
        let nanos = (i128::from(filetime) - 116_444_736_000_000_000) * 100;
        (
            nanos.div_euclid(1_000_000_000) as i64,
            nanos.rem_euclid(1_000_000_000) as i64,
        )
    }
    let (atime, ansec) = unix_time(metadata.last_access_time());
    let (mtime, mnsec) = unix_time(metadata.last_write_time());
    // Stable std exposes Windows creation time, not POSIX metadata-change time.
    let (ctime, cnsec) = unix_time(metadata.creation_time());
    crate::arch::Stat {
        dev: 0,
        ino: inode(path.as_bytes()),
        mode: mode(metadata),
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 0,
        size: metadata.len(),
        blksize: 4096,
        blocks: metadata.len().div_ceil(512),
        times: [atime, ansec, mtime, mnsec, ctime, cnsec],
    }
}
