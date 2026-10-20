//! A synchronous reader of [PMTiles v3] archives: one file with every tile
//! of a map, found through a directory of Hilbert tile ids. It reads with
//! `File` and `seek`, so the task pool can call it without async.
//!
//! [PMTiles v3]: https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;

use crate::geo::TileId;

const MAGIC: &[u8; 7] = b"PMTiles";
const HEADER_LEN: usize = 127;
/// Directories nest at most this deep (root and leaves).
const MAX_DEPTH: usize = 4;
/// Leaf directories kept decoded.
const LEAF_CACHE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    Unknown,
    None,
    Gzip,
    Brotli,
    Zstd,
}

impl Compression {
    fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::None,
            2 => Self::Gzip,
            3 => Self::Brotli,
            4 => Self::Zstd,
            _ => Self::Unknown,
        }
    }

    fn decode(self, bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        match self {
            Self::None => Ok(bytes),
            Self::Gzip => {
                let mut out = Vec::with_capacity(bytes.len() * 3);
                GzDecoder::new(bytes.as_slice()).read_to_end(&mut out)?;
                Ok(out)
            }
            other => Err(invalid(format!("compresión {other:?} no soportada"))),
        }
    }
}

/// Kind of the tiles in an archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileType {
    Unknown,
    Mvt,
    Png,
    Jpeg,
    Webp,
    Avif,
}

/// The fixed 127-byte header.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub root_offset: u64,
    pub root_length: u64,
    pub metadata_offset: u64,
    pub metadata_length: u64,
    pub leaf_offset: u64,
    pub leaf_length: u64,
    pub data_offset: u64,
    pub data_length: u64,
    pub internal_compression: Compression,
    pub tile_compression: Compression,
    pub tile_type: TileType,
    pub min_zoom: u8,
    pub max_zoom: u8,
    /// West, south, east, north, in degrees.
    pub bounds: [f64; 4],
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

impl Header {
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HEADER_LEN || &bytes[..7] != MAGIC {
            return Err(invalid("no es un archivo PMTiles"));
        }
        if bytes[7] != 3 {
            return Err(invalid(format!("PMTiles v{} no soportado", bytes[7])));
        }
        let u64_at = |at: usize| {
            u64::from_le_bytes(
                bytes[at..at + 8]
                    .try_into()
                    .expect("8 bytes del encabezado"),
            )
        };
        let degrees = |at: usize| {
            f64::from(i32::from_le_bytes(
                bytes[at..at + 4]
                    .try_into()
                    .expect("4 bytes del encabezado"),
            )) / 1e7
        };
        Ok(Self {
            root_offset: u64_at(8),
            root_length: u64_at(16),
            metadata_offset: u64_at(24),
            metadata_length: u64_at(32),
            leaf_offset: u64_at(40),
            leaf_length: u64_at(48),
            data_offset: u64_at(56),
            data_length: u64_at(64),
            internal_compression: Compression::from_byte(bytes[97]),
            tile_compression: Compression::from_byte(bytes[98]),
            tile_type: match bytes[99] {
                1 => TileType::Mvt,
                2 => TileType::Png,
                3 => TileType::Jpeg,
                4 => TileType::Webp,
                5 => TileType::Avif,
                _ => TileType::Unknown,
            },
            min_zoom: bytes[100],
            max_zoom: bytes[101],
            bounds: [degrees(102), degrees(106), degrees(110), degrees(114)],
        })
    }
}

/// The id of tile `z/x/y` along the Hilbert curves of every zoom.
pub fn tile_id(tile: TileId) -> u64 {
    let z = u32::from(tile.z);
    let mut acc = ((1u64 << (2 * z)) - 1) / 3;
    let (mut x, mut y) = (u64::from(tile.x), u64::from(tile.y));
    let n = 1u64 << z;
    let mut s = n >> 1;
    while s > 0 {
        let rx = u64::from(x & s > 0);
        let ry = u64::from(y & s > 0);
        acc += s * s * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = n - 1 - x;
                y = n - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    acc
}

/// One entry of a directory: a run of `run_length` tiles with the same
/// data, or (`run_length` 0) a leaf directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub tile_id: u64,
    pub offset: u64,
    pub length: u32,
    pub run_length: u32,
}

fn varint(bytes: &[u8], at: &mut usize) -> io::Result<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes
            .get(*at)
            .ok_or_else(|| invalid("directorio truncado"))?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(invalid("varint demasiado largo"))
}

/// A decompressed directory: ids as deltas, then run lengths, lengths and
/// offsets (0 meaning right after the previous entry).
pub fn decode_directory(bytes: &[u8]) -> io::Result<Vec<Entry>> {
    let mut at = 0;
    let count = usize::try_from(varint(bytes, &mut at)?).map_err(|_| invalid("directorio"))?;
    if count > bytes.len() {
        return Err(invalid("directorio con más entradas que bytes"));
    }
    let mut entries = vec![
        Entry {
            tile_id: 0,
            offset: 0,
            length: 0,
            run_length: 0,
        };
        count
    ];
    let mut last = 0u64;
    for entry in &mut entries {
        last += varint(bytes, &mut at)?;
        entry.tile_id = last;
    }
    let small = |value: u64| u32::try_from(value).map_err(|_| invalid("entrada fuera de rango"));
    for entry in &mut entries {
        entry.run_length = small(varint(bytes, &mut at)?)?;
    }
    for entry in &mut entries {
        entry.length = small(varint(bytes, &mut at)?)?;
    }
    for index in 0..count {
        let value = varint(bytes, &mut at)?;
        entries[index].offset = if value == 0 && index > 0 {
            let previous = entries[index - 1];
            previous.offset + u64::from(previous.length)
        } else {
            value.saturating_sub(1)
        };
    }
    Ok(entries)
}

/// The entry holding `id`: its run, or the leaf directory that may.
pub fn find_entry(entries: &[Entry], id: u64) -> Option<Entry> {
    let after = entries.partition_point(|entry| entry.tile_id <= id);
    let entry = *entries.get(after.checked_sub(1)?)?;
    if entry.tile_id == id
        || entry.run_length == 0
        || id - entry.tile_id < u64::from(entry.run_length)
    {
        Some(entry)
    } else {
        None
    }
}

/// An open archive. Reads lock the file, so threads can share it.
pub struct PmTiles {
    file: Mutex<File>,
    header: Header,
    root: Vec<Entry>,
    leaves: Mutex<HashMap<u64, Arc<Vec<Entry>>>>,
}

impl std::fmt::Debug for PmTiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmTiles")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

impl PmTiles {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut head = vec![0; HEADER_LEN];
        file.read_exact(&mut head)?;
        let header = Header::parse(&head)?;
        let mut archive = Self {
            file: Mutex::new(file),
            header,
            root: Vec::new(),
            leaves: Mutex::new(HashMap::new()),
        };
        archive.root = archive.directory(archive.header.root_offset, archive.header.root_length)?;
        Ok(archive)
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    fn read(&self, offset: u64, length: u64) -> io::Result<Vec<u8>> {
        let length = usize::try_from(length).map_err(|_| invalid("bloque demasiado grande"))?;
        let mut bytes = vec![0; length];
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn directory(&self, offset: u64, length: u64) -> io::Result<Vec<Entry>> {
        let bytes = self.read(offset, length)?;
        decode_directory(&self.header.internal_compression.decode(bytes)?)
    }

    fn leaf(&self, offset: u64, length: u64) -> io::Result<Arc<Vec<Entry>>> {
        let cached = self
            .leaves
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(&offset)
            .cloned();
        if let Some(leaf) = cached {
            return Ok(leaf);
        }
        let leaf = Arc::new(self.directory(self.header.leaf_offset + offset, length)?);
        let mut leaves = self
            .leaves
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if leaves.len() >= LEAF_CACHE {
            leaves.clear();
        }
        leaves.insert(offset, Arc::clone(&leaf));
        Ok(leaf)
    }

    /// The decompressed bytes of `tile`, or `None` when the archive has
    /// none there (outside its area or zooms).
    pub fn tile(&self, tile: TileId) -> io::Result<Option<Vec<u8>>> {
        if tile.z < self.header.min_zoom || tile.z > self.header.max_zoom || !tile.is_valid() {
            return Ok(None);
        }
        let id = tile_id(tile);
        let mut leaf: Option<Arc<Vec<Entry>>> = None;
        for _ in 0..MAX_DEPTH {
            let entries = leaf.as_deref().map_or(self.root.as_slice(), Vec::as_slice);
            let Some(entry) = find_entry(entries, id) else {
                return Ok(None);
            };
            if entry.run_length > 0 {
                let bytes = self.read(
                    self.header.data_offset + entry.offset,
                    u64::from(entry.length),
                )?;
                return self.header.tile_compression.decode(bytes).map(Some);
            }
            leaf = Some(self.leaf(entry.offset, u64::from(entry.length))?);
        }
        Err(invalid("directorios anidados demasiado profundo"))
    }
}

/// A minimal archive writer for tests: one root directory, gzip
/// everywhere, `tiles` in any order.
#[cfg(test)]
pub(crate) fn write_archive(tiles: &[(TileId, Vec<u8>)], max_zoom: u8) -> Vec<u8> {
    use std::io::Write;

    use flate2::Compression as Level;
    use flate2::write::GzEncoder;

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Level::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }
    fn put_varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    let mut sorted: Vec<(u64, Vec<u8>)> = tiles
        .iter()
        .map(|(tile, bytes)| (tile_id(*tile), gzip(bytes)))
        .collect();
    sorted.sort_by_key(|(id, _)| *id);
    let mut data = Vec::new();
    let mut directory = Vec::new();
    put_varint(&mut directory, sorted.len() as u64);
    let mut last = 0;
    for (id, _) in &sorted {
        put_varint(&mut directory, id - last);
        last = *id;
    }
    for _ in &sorted {
        put_varint(&mut directory, 1);
    }
    for (_, bytes) in &sorted {
        put_varint(&mut directory, bytes.len() as u64);
    }
    for (index, (_, bytes)) in sorted.iter().enumerate() {
        // Every tile right after the previous one: offset 0 but the first.
        put_varint(&mut directory, if index == 0 { 1 } else { 0 });
        data.extend_from_slice(bytes);
    }
    let root = gzip(&directory);

    let mut out = vec![0; HEADER_LEN];
    out[..7].copy_from_slice(MAGIC);
    out[7] = 3;
    let root_offset = HEADER_LEN as u64;
    let data_offset = root_offset + root.len() as u64;
    for (at, value) in [
        (8, root_offset),
        (16, root.len() as u64),
        (24, data_offset),
        (32, 0),
        (40, data_offset),
        (48, 0),
        (56, data_offset),
        (64, data.len() as u64),
        (72, sorted.len() as u64),
        (80, sorted.len() as u64),
        (88, sorted.len() as u64),
    ] {
        out[at..at + 8].copy_from_slice(&u64::to_le_bytes(value));
    }
    out[96] = 1;
    out[97] = 2;
    out[98] = 2;
    out[99] = 1;
    out[100] = 0;
    out[101] = max_zoom;
    for (at, degrees) in [(102, -180.0), (106, -85.0), (110, 180.0), (114, 85.0)] {
        out[at..at + 4].copy_from_slice(&i32::to_le_bytes((degrees * 1e7_f64) as i32));
    }
    out.extend_from_slice(&root);
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_ids_follow_the_spec() {
        let ids: Vec<u64> = [
            (0, 0, 0),
            (1, 0, 0),
            (1, 0, 1),
            (1, 1, 1),
            (1, 1, 0),
            (2, 0, 0),
        ]
        .iter()
        .map(|&(z, x, y)| tile_id(TileId::new(z, x, y)))
        .collect();
        assert_eq!(ids, [0, 1, 2, 3, 4, 5]);
        assert_eq!(tile_id(TileId::new(3, 7, 0)), 84);
        assert_eq!(tile_id(TileId::new(20, 0, 0)), 366_503_875_925);
    }

    #[test]
    fn runs_and_leaves_are_found() {
        let entries = [
            Entry {
                tile_id: 5,
                offset: 0,
                length: 10,
                run_length: 3,
            },
            Entry {
                tile_id: 20,
                offset: 10,
                length: 4,
                run_length: 0,
            },
        ];
        assert_eq!(find_entry(&entries, 4), None);
        assert_eq!(find_entry(&entries, 7), Some(entries[0]));
        assert_eq!(find_entry(&entries, 8), None, "past the run");
        assert_eq!(find_entry(&entries, 99), Some(entries[1]), "a leaf");
    }

    #[test]
    fn an_archive_written_here_reads_back() {
        let tiles = [
            (TileId::new(0, 0, 0), b"mundo".to_vec()),
            (TileId::new(13, 2461, 4979), b"plaza".to_vec()),
            (TileId::new(2, 1, 2), b"sur".to_vec()),
        ];
        let path = std::env::temp_dir().join(format!("kai-pmtiles-{}.pmtiles", std::process::id()));
        std::fs::write(&path, write_archive(&tiles, 14)).unwrap();
        let archive = PmTiles::open(&path).unwrap();
        assert_eq!(archive.header().tile_type, TileType::Mvt);
        assert_eq!(archive.header().max_zoom, 14);
        for (tile, bytes) in &tiles {
            assert_eq!(archive.tile(*tile).unwrap().as_ref(), Some(bytes));
        }
        assert_eq!(archive.tile(TileId::new(13, 2461, 4980)).unwrap(), None);
        assert_eq!(
            archive.tile(TileId::new(15, 0, 0)).unwrap(),
            None,
            "beyond max zoom"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn the_parral_extract_has_the_plaza() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/maps/parral.pmtiles");
        let archive = PmTiles::open(&path).unwrap();
        let header = archive.header();
        assert_eq!(header.tile_type, TileType::Mvt);
        assert_eq!((header.min_zoom, header.max_zoom), (0, 15));
        assert!(
            (header.bounds[0] + 71.95).abs() < 1e-6,
            "{:?}",
            header.bounds
        );
        let plaza = archive.tile(TileId::new(15, 9846, 19916)).unwrap();
        assert!(plaza.is_some_and(|bytes| bytes.len() > 1_000));
        assert_eq!(archive.tile(TileId::new(15, 0, 0)).unwrap(), None);
    }
}
