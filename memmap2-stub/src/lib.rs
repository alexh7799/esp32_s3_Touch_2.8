//! memmap2 stub for ESP-IDF / Xtensa targets.
//! mmap is not available on ESP-IDF; this stub satisfies the API surface
//! required by fontdb without calling any OS memory-mapping functions.

use std::ops::Deref;
use std::fs::File;

/// Stub for `MmapOptions`. No actual memory mapping is performed.
#[derive(Default)]
pub struct MmapOptions;

impl MmapOptions {
    /// Creates a new `MmapOptions` with default settings.
    pub fn new() -> Self {
        Self
    }

    /// Stub: always returns an error on ESP-IDF.
    pub unsafe fn map(&self, _file: &File) -> std::io::Result<Mmap> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "mmap is not supported on ESP-IDF",
        ))
    }
}

/// Stub for a read-only memory map.
pub struct Mmap {
    data: Vec<u8>,
}

impl Mmap {
    /// Stub: maps a file by reading it into memory instead.
    ///
    /// # Safety
    /// Safe on ESP-IDF because no actual mmap is used.
    pub unsafe fn map(file: &File) -> std::io::Result<Self> {
        use std::io::Read;
        let mut data = Vec::new();
        let mut f = file.try_clone()?;
        f.read_to_end(&mut data)?;
        Ok(Self { data })
    }
}

impl Deref for Mmap {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}

impl AsRef<[u8]> for Mmap {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

/// Stub for a read-write memory map.
pub struct MmapMut {
    data: Vec<u8>,
}

impl MmapMut {
    /// Stub: always returns an error on ESP-IDF.
    pub unsafe fn map_mut(_file: &File) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "mmap_mut is not supported on ESP-IDF",
        ))
    }

    /// Creates an anonymous mutable mapping backed by a Vec.
    pub fn map_anon(len: usize) -> std::io::Result<Self> {
        Ok(Self { data: vec![0u8; len] })
    }

    /// Flushes the mapping (no-op on this stub).
    pub fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }

    /// Freezes the mutable map into a read-only map.
    pub fn make_read_only(self) -> std::io::Result<Mmap> {
        Ok(Mmap { data: self.data })
    }
}

impl Deref for MmapMut {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}

impl std::ops::DerefMut for MmapMut {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl AsRef<[u8]> for MmapMut {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for MmapMut {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}