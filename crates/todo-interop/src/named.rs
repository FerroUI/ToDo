//! MS-PST name-to-ID mappings are store specific: LIDs are never property IDs.
use crate::{InteropError, Result};
use outlook_pst::ltp::prop_context::PropertyValue;
use outlook_pst::messaging::store::Store;
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) const TASK: &str = "00062003-0000-0000-c000-000000000046";
pub(crate) const COMMON: &str = "00062008-0000-0000-c000-000000000046";
pub(crate) const PUBLIC: &str = "00020329-0000-0000-c000-000000000046";

#[derive(Serialize)]
pub(crate) struct Name {
    pub guid: String,
    pub name: String,
}
#[derive(Default)]
pub(crate) struct Names(pub BTreeMap<u16, Name>);
impl Names {
    pub(crate) fn load(store: &dyn Store) -> Result<Self> {
        let map = store.named_property_map()?;
        let props = map.properties();
        let binary = |id| match props.get(id) {
            Some(PropertyValue::Binary(value)) => value.buffer(),
            _ => &[],
        };
        Self::parse(binary(2), binary(3), binary(4))
    }
    fn parse(guids: &[u8], entries: &[u8], strings: &[u8]) -> Result<Self> {
        if guids.len() % 16 != 0 || entries.len() % 8 != 0 || entries.len() / 8 > 32768 {
            return Err(InteropError::new("The PST named-property map is invalid."));
        }
        let mut names = BTreeMap::new();
        for entry in entries.chunks_exact(8) {
            let value = u32::from_le_bytes(entry[..4].try_into().unwrap());
            let kind = u16::from_le_bytes(entry[4..6].try_into().unwrap());
            let index = u16::from_le_bytes(entry[6..8].try_into().unwrap());
            if index >= 0x8000 {
                return Err(InteropError::new(
                    "The PST named-property index is invalid.",
                ));
            }
            let guid = match kind >> 1 {
                0 => "00000000-0000-0000-0000-000000000000".into(),
                1 => "00020328-0000-0000-c000-000000000046".into(),
                2 => PUBLIC.into(),
                n => {
                    let start = usize::from(n - 3) * 16;
                    let bytes = guids.get(start..start + 16).ok_or_else(|| {
                        InteropError::new("The PST named-property GUID is missing.")
                    })?;
                    guid_string(bytes)
                }
            };
            let name = if kind & 1 == 0 {
                format!("{value:08x}")
            } else {
                let start = value as usize;
                let length = strings
                    .get(start..start.saturating_add(4))
                    .ok_or_else(|| InteropError::new("The PST named-property name is missing."))?;
                let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
                let bytes = strings
                    .get(start + 4..(start + 4).saturating_add(length))
                    .ok_or_else(|| {
                        InteropError::new("The PST named-property name is truncated.")
                    })?;
                if length % 2 != 0 {
                    return Err(InteropError::new("A PST property name is not UTF-16."));
                }
                String::from_utf16(
                    &bytes
                        .chunks_exact(2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .collect::<Vec<_>>(),
                )
                .map_err(|_| InteropError::new("A PST property name contains invalid UTF-16."))?
            };
            if names.insert(index + 0x8000, Name { guid, name }).is_some() {
                return Err(InteropError::new(
                    "The PST named-property map contains duplicate IDs.",
                ));
            }
        }
        Ok(Self(names))
    }
    pub(crate) fn id(&self, guid: &str, lid: u32) -> Option<u16> {
        self.find(guid, &format!("{lid:08x}"))
    }
    pub(crate) fn find(&self, guid: &str, name: &str) -> Option<u16> {
        self.0
            .iter()
            .find_map(|(id, entry)| (entry.guid == guid && entry.name == name).then_some(*id))
    }
}

fn guid_string(bytes: &[u8]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_task_lids_through_store_specific_property_ids() {
        let guid = [3, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70];
        let names = Names::parse(&guid, &[5, 129, 0, 0, 6, 0, 70, 0], &[]).unwrap();
        assert_eq!(names.id(TASK, 0x8105), Some(0x8046));
        assert_eq!(names.id(COMMON, 0x8105), None);
    }
    #[test]
    fn malformed_string_length_fails_without_allocation() {
        let entry = [0, 0, 0, 0, 5, 0, 0, 0];
        assert!(Names::parse(&[], &entry, &[255, 255, 255, 255]).is_err());
        assert!(Names::parse(&[], &[0; 7], &[]).is_err());
    }
    #[test]
    fn oversized_named_map_and_duplicate_indices_fail() {
        assert!(Names::parse(&[], &vec![0; 32769 * 8], &[]).is_err());
        let entry = [5, 129, 0, 0, 4, 0, 7, 0];
        assert!(Names::parse(&[], &[entry, entry].concat(), &[]).is_err());
    }
}
