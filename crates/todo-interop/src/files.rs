use crate::{
    properties::{integer, text},
    ArchiveFile, InteropError, Result, MAX_FILES_BYTES, MAX_FILE_BYTES,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use outlook_pst::messaging::{
    attachment::{AnsiAttachment, Attachment, AttachmentData, UnicodeAttachment},
    message::{AnsiMessage, Message, UnicodeMessage},
};
use sha2::{Digest, Sha256};
use std::rc::Rc;

pub(crate) enum PstMessage {
    Ansi(Rc<AnsiMessage>),
    Unicode(Rc<UnicodeMessage>),
}
impl PstMessage {
    pub(crate) fn message(&self) -> &dyn Message {
        match self {
            Self::Ansi(value) => value.as_ref(),
            Self::Unicode(value) => value.as_ref(),
        }
    }
    fn attachment(&self, id: u32) -> Result<Rc<dyn Attachment>> {
        Ok(match self {
            Self::Ansi(message) => AnsiAttachment::read(message.clone(), id.into(), None)?,
            Self::Unicode(message) => UnicodeAttachment::read(message.clone(), id.into(), None)?,
        })
    }
}

pub(crate) fn append(
    message: &PstMessage,
    document: &mut todo_domain::AppState,
    task: &mut todo_domain::Task,
    files: &mut Vec<ArchiveFile>,
    total: &mut usize,
) -> Result<()> {
    let Some(table) = message.message().attachment_table() else {
        return Ok(());
    };
    let mut seen = std::collections::HashSet::new();
    for row in table.rows_matrix() {
        if files.len() >= crate::MAX_ITEMS {
            return Err(InteropError::new("The PST contains too many attachments."));
        }
        let node = u32::from(row.id());
        if !seen.insert(node) {
            return Err(InteropError::new("The PST repeats an attachment identity."));
        }
        let attachment = message.attachment(node)?;
        let props = attachment.properties();
        let name = text(props.get(0x3707), 1252)?
            .or(text(props.get(0x3704), 1252)?)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "attachment".into());
        if integer(props.get(0x3705)) != Some(1) {
            return Err(InteropError::new(format!("“{name}” is an embedded Outlook item or linked attachment. Export it as a regular file in Outlook before importing this PST.")));
        }
        let Some(AttachmentData::Binary(data)) = attachment.data() else {
            return Err(InteropError::new(format!(
                "“{name}” has no complete file contents."
            )));
        };
        let bytes = data.buffer();
        if bytes.len() > MAX_FILE_BYTES {
            return Err(InteropError::new(format!(
                "“{name}” exceeds the 25 MiB attachment limit."
            )));
        }
        *total = total
            .checked_add(bytes.len())
            .ok_or_else(|| InteropError::new("PST attachment sizes overflowed."))?;
        if *total > MAX_FILES_BYTES {
            return Err(InteropError::new(
                "The PST tasks contain more than 128 MiB of attachment contents.",
            ));
        }
        let mime_type = text(props.get(0x370e), 1252)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "application/octet-stream".into());
        let key = format!("pst:{}", files.len() + 1);
        let id = document.next_id;
        document.next_id += 1;
        task.attachments.push(todo_domain::Attachment {
            id,
            name: name.clone(),
            mime_type: mime_type.clone(),
            size: bytes.len() as u64,
            storage_key: key.clone(),
        });
        files.push(ArchiveFile {
            key,
            name,
            mime_type,
            size: bytes.len(),
            base64: STANDARD.encode(bytes),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        });
    }
    Ok(())
}
