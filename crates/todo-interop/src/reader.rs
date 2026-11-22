use crate::{InteropError, Result};
use outlook_pst::messaging::store::{AnsiStore, Store, UnicodeStore};
use outlook_pst::ndb::{byte_index::ByteIndex, header::Header, root::Root};
use outlook_pst::{AnsiPstFile, PstFile, UnicodePstFile};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;

pub(crate) enum PstStore {
    Ansi(Rc<AnsiStore>),
    Unicode(Rc<UnicodeStore>),
}
impl PstStore {
    pub(crate) fn store(&self) -> Rc<dyn Store> {
        match self {
            Self::Ansi(store) => store.clone(),
            Self::Unicode(store) => store.clone(),
        }
    }
}

// Limit I/O amplification, including a cyclic/corrupt B-tree. Seeks outside the
// supplied file fail immediately, rather than permitting fabricated offsets.
struct BoundedReader {
    inner: Cursor<Vec<u8>>,
    operations: usize,
    bytes_left: u64,
}
impl BoundedReader {
    fn new(bytes: Vec<u8>) -> Self {
        let bytes_left = (bytes.len() as u64).saturating_mul(32).max(1_048_576);
        Self {
            inner: Cursor::new(bytes),
            operations: 4_000_000,
            bytes_left,
        }
    }
    fn charge(&mut self) -> io::Result<()> {
        self.operations = self
            .operations
            .checked_sub(1)
            .ok_or_else(|| io::Error::other("PST parser exceeded its operation limit"))?;
        Ok(())
    }
}
impl Read for BoundedReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.charge()?;
        let available = self.inner.get_ref().len() as u64 - self.inner.position();
        let amount = available.min(out.len() as u64);
        self.bytes_left = self
            .bytes_left
            .checked_sub(amount)
            .ok_or_else(|| io::Error::other("PST parser exceeded its read limit"))?;
        self.inner.read(out)
    }
}
impl Seek for BoundedReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.charge()?;
        let size = self.inner.get_ref().len() as i128;
        let target = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.inner.position() as i128 + value as i128,
            SeekFrom::End(value) => size + value as i128,
        };
        if !(0..=size).contains(&target) {
            return Err(io::Error::other("PST offset is outside the file"));
        }
        self.inner.seek(SeekFrom::Start(target as u64))
    }
}

pub(crate) fn open(bytes: Vec<u8>) -> Result<PstStore> {
    if bytes.len() < 512 || &bytes[..4] != b"!BDN" || &bytes[8..10] != b"SM" {
        return Err(InteropError::new("This file is not an Outlook PST."));
    }
    let length = bytes.len();
    let version = u16::from_le_bytes([bytes[10], bytes[11]]);
    let reader = Box::new(BoundedReader::new(bytes));
    match version {
        14 | 15 => {
            let pst=AnsiPstFile::read_from(reader)?; check_size(&pst,length)?;
            Ok(PstStore::Ansi(AnsiStore::read(Rc::new(pst))?))
        },
        23 => {
            let pst=UnicodePstFile::read_from(reader)?; check_size(&pst,length)?;
            Ok(PstStore::Unicode(UnicodeStore::read(Rc::new(pst))?))
        },
        _ => Err(InteropError::new(format!("Outlook PST version {version} is unsupported. Export an Outlook 2003–2019 Unicode PST (version 23). OST/4K stores are not supported."))),
    }
}

fn check_size<P: PstFile>(pst: &P, length: usize) -> Result<()> {
    let expected: u64 = pst.header().root().file_eof_index().index().into();
    if expected < 512 || expected > length as u64 {
        return Err(InteropError::new(
            "The Outlook PST is truncated or has an invalid declared length.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_header_and_unsupported_stores_are_rejected() {
        assert!(open(b"{}".to_vec()).is_err());
        let mut bytes = vec![0; 512];
        bytes[..4].copy_from_slice(b"!BDN");
        bytes[8..10].copy_from_slice(b"SM");
        bytes[10] = 36;
        assert!(open(bytes)
            .err()
            .unwrap()
            .to_string()
            .contains("version 36"));
    }
    #[test]
    fn invalid_offsets_and_io_amplification_fail() {
        let mut reader = BoundedReader::new(vec![1, 2, 3]);
        assert!(reader.seek(SeekFrom::Start(4)).is_err());
        assert!(reader.seek(SeekFrom::Current(-1)).is_err());
        reader.operations = 1;
        reader.seek(SeekFrom::Start(0)).unwrap();
        assert!(reader.read(&mut [0; 1]).is_err());
    }
}
