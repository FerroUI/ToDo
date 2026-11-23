use ferroui_base::media::imaging::{Bitmap, BitmapInterpolationMode, JpegBitmapEncoderOptions};
use ferroui_base::platform::storage::{FilePickerFileType, FilePickerOpenOptions};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Ref};
use ferroui_controls::TopLevel;
use std::io::{Cursor, Read};
use std::rc::Rc;
use todo_domain::backgrounds::{
    encode_background, image_dimensions, MAX_BACKGROUND_BYTES, MAX_BACKGROUND_SIDE,
};
use todo_state::{AppViewModel, Panel};

const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCE_PIXELS: u64 = 12_000_000;

pub(super) fn choose(vm: Rc<AppViewModel>, root: Ref<TopLevel>) {
    // Revision is monotonic across imports/account switches, even when two
    // accounts happen to use identical local entity IDs and document contents.
    let expected_revision = vm.revision();
    let expected_view = vm.view();
    Dispatcher::ui_thread().spawn_local(async move {
        let result = async {
            let mut options = FilePickerOpenOptions::new();
            options.set_title(Some("Choose a list background".into()));
            options.set_file_type_filter(Some(vec![FilePickerFileType::new(Some("JPEG or PNG image"))
                .with_patterns(&["*.jpg", "*.jpeg", "*.png"]).with_mime_types(&["image/jpeg", "image/png"])]));
            let files = root.storage_provider().open_file_picker_async(options).await.map_err(|e| e.to_string())?;
            let Some(file) = files.first() else { return Ok(None); };
            let size = file.get_basic_properties_async().await.size().ok_or("Cannot determine this image's file size")?;
            if size == 0 || size > MAX_SOURCE_BYTES { return Err("Choose a JPEG or PNG image up to 8 MiB".into()); }
            let input = file.open_read_async().await.map_err(|e| e.to_string())?;
            let mut bytes = Vec::with_capacity(size as usize);
            input.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_SOURCE_BYTES { return Err("The selected image exceeds 8 MiB".into()); }
            let (width, height) = image_dimensions(&bytes).ok_or("Choose a valid JPEG or PNG image")?;
            if width > 8192 || height > 8192 || u64::from(width) * u64::from(height) > MAX_SOURCE_PIXELS {
                return Err("Choose an image up to 12 megapixels and 8192 pixels on either side".into());
            }
            if vm.revision() != expected_revision || vm.view() != expected_view {
                return Err("The list or account changed while choosing an image. Choose it again for the current list.".into());
            }
            let target_width = ((width as f64 * (MAX_BACKGROUND_SIDE as f64 / width.max(height) as f64).min(1.0)).round() as i32).max(1);
            let bitmap = Bitmap::decode_to_width(&mut Cursor::new(bytes), target_width, BitmapInterpolationMode::HighQuality).map_err(|e| e.to_string())?;
            let size = bitmap.pixel_size();
            if size.width <= 0 || size.height <= 0 || size.width > MAX_BACKGROUND_SIDE as i32 || size.height > MAX_BACKGROUND_SIDE as i32 {
                return Err("The image decoder returned unsupported dimensions".into());
            }
            for scale in [1.0, 0.8, 0.6, 0.4] {
                let scaled = bitmap.create_scaled_bitmap(PixelSize::new((size.width as f64 * scale).round().max(1.0) as i32, (size.height as f64 * scale).round().max(1.0) as i32), BitmapInterpolationMode::HighQuality);
                for quality in [84, 70, 55] {
                    let mut bytes = Vec::new();
                    scaled.save(&mut bytes, &JpegBitmapEncoderOptions { quality }.into()).map_err(|e| e.to_string())?;
                    if bytes.len() <= MAX_BACKGROUND_BYTES { return encode_background(&bytes).map(Some).map_err(str::to_owned); }
                }
            }
            Err("This image cannot be resized within the background size limit".into())
        }.await;
        match result {
            Ok(Some(value)) if vm.revision() == expected_revision && vm.view() == expected_view => {
                vm.set_wallpaper(Some(value)); vm.show_panel(Panel::None);
            }
            Ok(Some(_)) => vm.report_error("The list or account changed while choosing an image. Choose it again for the current list.".into()),
            Ok(None) => {}
            Err(error) => vm.report_error(error),
        }
    });
}
