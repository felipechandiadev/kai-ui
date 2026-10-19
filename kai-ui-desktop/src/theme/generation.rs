//! Which AppKit generation the native controls of this process show
//! (`docs/dev/11-tema-appkit.md` 6.3.8). AppKit draws the 26+ design only
//! for binaries linked with SDK 26 or later (`sdk` field of
//! `LC_BUILD_VERSION`); with an older SDK the windows, menus and controls of
//! the same process keep the macOS 15 look even on 27. Kai imitates those
//! native parts, so it takes the lower of the OS and the linked SDK.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::OnceLock;

use super::AppKitGeneration;

/// `major.minor` of macOS or of a macOS SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MacVersion {
    pub major: u32,
    pub minor: u32,
}

impl MacVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Mach-O `xxxx.yy.zz` nibble encoding (`0x001b_0000` = 27.0).
    const fn from_packed(packed: u32) -> Self {
        Self::new(packed >> 16, (packed >> 8) & 0xff)
    }
}

impl fmt::Display for MacVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// What [`AppKitGeneration::current`] decides from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemInfo {
    /// `NSProcessInfo.operatingSystemVersion`; `None` outside macOS. It
    /// reports the real version (27.0.1) also to SDK 15 binaries.
    pub os: Option<MacVersion>,
    /// SDK recorded in our own executable; `None` if it cannot be read.
    pub sdk: Option<MacVersion>,
}

impl SystemInfo {
    /// Reads the OS version and the linked SDK once per process.
    pub fn detect() -> SystemInfo {
        static INFO: OnceLock<SystemInfo> = OnceLock::new();
        *INFO.get_or_init(|| SystemInfo {
            os: os_version(),
            sdk: linked_sdk(),
        })
    }

    /// Generation of the native controls. Without macOS there is nothing
    /// to imitate: `Aqua11`. An unreadable SDK is assumed to match the OS.
    pub fn generation(self) -> AppKitGeneration {
        let Some(os) = self.os else {
            return AppKitGeneration::Aqua11;
        };
        let major = self.sdk.map_or(os.major, |sdk| sdk.major.min(os.major));
        match major {
            27.. => AppKitGeneration::Glass27,
            26 => AppKitGeneration::Glass26,
            _ => AppKitGeneration::Aqua11,
        }
    }

    /// The system tones (`systemRed`…) changed on 26+ for every app,
    /// whatever its SDK (11 §6.3.8): they follow the OS, not the generation.
    pub fn has_hig26_tones(self) -> bool {
        self.os.is_some_and(|os| os.major >= 26)
    }
}

impl AppKitGeneration {
    /// Generation of this process on this Mac ([`SystemInfo::generation`]).
    pub fn current() -> AppKitGeneration {
        SystemInfo::detect().generation()
    }
}

#[cfg(target_os = "macos")]
fn os_version() -> Option<MacVersion> {
    let v = objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion();
    Some(MacVersion::new(
        u32::try_from(v.majorVersion).ok()?,
        u32::try_from(v.minorVersion).ok()?,
    ))
}

#[cfg(not(target_os = "macos"))]
fn os_version() -> Option<MacVersion> {
    None
}

/// Load commands sit at the start of the image; 64 KiB covers them.
const HEAD: u64 = 64 * 1024;

fn linked_sdk() -> Option<MacVersion> {
    let mut file = File::open(std::env::current_exe().ok()?).ok()?;
    let mut head = Vec::new();
    (&mut file).take(HEAD).read_to_end(&mut head).ok()?;
    if let Some(offset) = fat_slice_offset(&head, NATIVE_CPU) {
        file.seek(SeekFrom::Start(offset)).ok()?;
        head.clear();
        file.take(HEAD).read_to_end(&mut head).ok()?;
    }
    macho_sdk(&head)
}

const MH_MAGIC_64: u32 = 0xfeed_facf;
const FAT_MAGIC: u32 = 0xcafe_babe;
const LC_VERSION_MIN_MACOSX: u32 = 0x24;
const LC_BUILD_VERSION: u32 = 0x32;
const PLATFORM_MACOS: u32 = 1;

#[cfg(target_arch = "aarch64")]
const NATIVE_CPU: u32 = 0x0100_000c;
#[cfg(target_arch = "x86_64")]
const NATIVE_CPU: u32 = 0x0100_0007;
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
const NATIVE_CPU: u32 = 0;

fn word(bytes: &[u8], at: usize, from: fn([u8; 4]) -> u32) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(from(bytes.get(at..end)?.try_into().ok()?))
}

/// Offset of the `cpu` slice of a universal binary; `None` if `bytes` is
/// not one (fat headers are big-endian).
fn fat_slice_offset(bytes: &[u8], cpu: u32) -> Option<u64> {
    let be = |at| word(bytes, at, u32::from_be_bytes);
    if be(0)? != FAT_MAGIC {
        return None;
    }
    (0..be(4)? as usize)
        .map(|i| 8 + 20 * i)
        .find(|&at| be(at) == Some(cpu))
        .and_then(|at| be(at + 8))
        .map(u64::from)
}

/// SDK of a thin 64-bit Mach-O image from its first bytes: the `sdk` of
/// `LC_BUILD_VERSION` (macOS platform) or of `LC_VERSION_MIN_MACOSX`.
fn macho_sdk(bytes: &[u8]) -> Option<MacVersion> {
    let le = |at| word(bytes, at, u32::from_le_bytes);
    if le(0)? != MH_MAGIC_64 {
        return None;
    }
    let mut at = 32;
    for _ in 0..le(16)? {
        let (cmd, size) = (le(at)?, le(at + 4)? as usize);
        match cmd {
            LC_BUILD_VERSION if le(at + 8)? == PLATFORM_MACOS => {
                return Some(MacVersion::from_packed(le(at + 16)?));
            }
            LC_VERSION_MIN_MACOSX => return Some(MacVersion::from_packed(le(at + 12)?)),
            _ => {}
        }
        if size < 8 {
            return None;
        }
        at = at.checked_add(size)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use AppKitGeneration::{Aqua11, Glass26, Glass27};

    fn info(os: Option<(u32, u32)>, sdk: Option<(u32, u32)>) -> SystemInfo {
        let v = |(major, minor)| MacVersion::new(major, minor);
        SystemInfo {
            os: os.map(v),
            sdk: sdk.map(v),
        }
    }

    #[test]
    fn generation_takes_the_lower_of_os_and_sdk() {
        let table = [
            // This Mac today: macOS 27 with the Command Line Tools 15.5 SDK.
            (Some((27, 0)), Some((15, 5)), Aqua11),
            (Some((27, 0)), Some((27, 0)), Glass27),
            (Some((28, 1)), Some((27, 0)), Glass27),
            (Some((27, 0)), Some((26, 2)), Glass26),
            (Some((26, 1)), Some((27, 0)), Glass26),
            (Some((26, 0)), Some((26, 0)), Glass26),
            (Some((15, 6)), Some((26, 0)), Aqua11),
            (Some((15, 6)), Some((15, 5)), Aqua11),
            (Some((11, 0)), None, Aqua11),
            // Unreadable SDK: follow the OS.
            (Some((27, 0)), None, Glass27),
            (Some((26, 0)), None, Glass26),
            // Not macOS.
            (None, Some((27, 0)), Aqua11),
            (None, None, Aqua11),
        ];
        for (os, sdk, expected) in table {
            assert_eq!(
                info(os, sdk).generation(),
                expected,
                "os {os:?} sdk {sdk:?}"
            );
        }
    }

    #[test]
    fn tones_follow_the_os_only() {
        assert!(info(Some((27, 0)), Some((15, 5))).has_hig26_tones());
        assert!(info(Some((26, 0)), None).has_hig26_tones());
        assert!(!info(Some((15, 6)), Some((26, 0))).has_hig26_tones());
        assert!(!info(None, None).has_hig26_tones());
    }

    /// Thin 64-bit Mach-O header followed by `cmds` (cmd, payload words).
    fn macho(cmds: &[(u32, &[u32])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (cmd, payload) in cmds {
            let words = [*cmd, 8 + 4 * payload.len() as u32]
                .into_iter()
                .chain(payload.iter().copied());
            body.extend(words.flat_map(u32::to_le_bytes));
        }
        let header = [
            MH_MAGIC_64,
            NATIVE_CPU,
            0,
            2,
            cmds.len() as u32,
            body.len() as u32,
            0,
            0,
        ];
        let mut bytes: Vec<u8> = header.into_iter().flat_map(u32::to_le_bytes).collect();
        bytes.extend(body);
        bytes
    }

    #[test]
    fn reads_the_sdk_of_lc_build_version() {
        let segment = (0x19, &[0u32; 16][..]);
        // platform, minos 11.0, sdk 15.5, ntools 0.
        let build = (
            LC_BUILD_VERSION,
            &[PLATFORM_MACOS, 0x000b_0000, 0x000f_0500, 0][..],
        );
        assert_eq!(
            macho_sdk(&macho(&[segment, build])),
            Some(MacVersion::new(15, 5))
        );
        let build27 = (
            LC_BUILD_VERSION,
            &[PLATFORM_MACOS, 0x000b_0000, 0x001b_0000, 0][..],
        );
        assert_eq!(macho_sdk(&macho(&[build27])), Some(MacVersion::new(27, 0)));
        // Mac Catalyst (platform 6) is skipped.
        let catalyst = (LC_BUILD_VERSION, &[6, 0x000e_0000, 0x001a_0000, 0][..]);
        assert_eq!(macho_sdk(&macho(&[catalyst])), None);
        let min = (LC_VERSION_MIN_MACOSX, &[0x000a_0f00, 0x000a_0e00][..]);
        assert_eq!(macho_sdk(&macho(&[min])), Some(MacVersion::new(10, 14)));
        assert_eq!(macho_sdk(&macho(&[segment])), None);
        assert_eq!(macho_sdk(b"not a binary"), None);
        let truncated = macho(&[segment, build]);
        assert_eq!(macho_sdk(&truncated[..60]), None);
    }

    #[test]
    fn finds_the_native_slice_of_a_universal_binary() {
        let words = [
            FAT_MAGIC,
            2,
            0x0100_0007,
            3,
            0x4000,
            0x100,
            14,
            0x0100_000c,
            0,
            0x8000,
            0x100,
            14,
        ];
        let fat: Vec<u8> = words.into_iter().flat_map(u32::to_be_bytes).collect();
        assert_eq!(fat_slice_offset(&fat, 0x0100_000c), Some(0x8000));
        assert_eq!(fat_slice_offset(&fat, 0x0100_0007), Some(0x4000));
        assert_eq!(fat_slice_offset(&fat, 0x0000_0012), None);
        assert_eq!(fat_slice_offset(&macho(&[]), NATIVE_CPU), None);
        assert_eq!(MacVersion::new(27, 0).to_string(), "27.0");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_this_mac() {
        let info = SystemInfo::detect();
        let os = info.os.expect("macOS version");
        let sdk = info.sdk.expect("SDK of the test binary");
        assert!(os >= MacVersion::new(11, 0), "{os}");
        assert!(sdk >= MacVersion::new(11, 0), "{sdk}");
        assert_eq!(AppKitGeneration::current(), info.generation());
    }
}
