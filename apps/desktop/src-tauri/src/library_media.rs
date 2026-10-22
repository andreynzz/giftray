// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Restricted WebView media protocol for persistent-library previews.

use std::io::Cursor;

use image::{ImageFormat, ImageReader, Limits};
use tauri::http::{Response, StatusCode};
use uuid::Uuid;

use crate::library::{Library, MediaFormat};

const THUMBNAIL_WIDTH: u32 = 240;
const THUMBNAIL_HEIGHT: u32 = 160;
const MAX_THUMBNAIL_ALLOCATION: u64 = 16 * 1024 * 1024;

pub(crate) fn respond(library: &Library, path: &str) -> Response<Vec<u8>> {
    let Some((kind, id)) = path
        .strip_prefix('/')
        .and_then(|value| value.split_once('/'))
        .filter(|(_, id)| !id.contains('/'))
    else {
        return not_found();
    };
    if !matches!(kind, "preview" | "thumbnail") || Uuid::parse_str(id).is_err() {
        return not_found();
    }
    let Ok(media) = library.read_media_for_preview(id) else {
        return not_found();
    };
    if kind == "preview" {
        return response(StatusCode::OK, media_type(media.format), media.bytes);
    }
    match thumbnail(&media.bytes, media.format) {
        Ok(bytes) => response(StatusCode::OK, "image/png", bytes),
        Err(()) => unavailable(),
    }
}

fn thumbnail(bytes: &[u8], format: MediaFormat) -> Result<Vec<u8>, ()> {
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(match format {
        MediaFormat::Gif => ImageFormat::Gif,
        MediaFormat::Webp => ImageFormat::WebP,
        MediaFormat::Png => ImageFormat::Png,
    });
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_THUMBNAIL_ALLOCATION);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| ())?;
    let thumbnail = image.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT);
    let mut output = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut output), ImageFormat::Png)
        .map_err(|_| ())?;
    Ok(output)
}

fn media_type(format: MediaFormat) -> &'static str {
    match format {
        MediaFormat::Gif => "image/gif",
        MediaFormat::Webp => "image/webp",
        MediaFormat::Png => "image/png",
    }
}

fn response(status: StatusCode, content_type: &'static str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn not_found() -> Response<Vec<u8>> {
    response(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        b"Not found".to_vec(),
    )
}

fn unavailable() -> Response<Vec<u8>> {
    response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "text/plain; charset=utf-8",
        b"Unavailable".to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::library::ImportResult;

    fn root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("giftray-media-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn imported(result: ImportResult) -> crate::library::LibraryItem {
        match result {
            ImportResult::Imported { item } => item,
            _ => panic!("expected imported item"),
        }
    }

    #[test]
    fn serves_valid_media_and_rejects_invalid_or_traversal_paths() {
        let root = root("protocol");
        let source = root.join("item.gif");
        fs::write(&source, b"GIF89a preview").unwrap();
        let library = Library::initialize(&root).unwrap();
        let item = imported(library.import_local_file(&source).unwrap());

        let response = respond(&library, &format!("/preview/{}", item.id));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/gif");
        assert_eq!(response.body(), b"GIF89a preview");
        assert_eq!(
            respond(&library, "/preview/not-a-uuid").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            respond(&library, "/preview/../library.sqlite3").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            respond(&library, "/preview/anything/else").status(),
            StatusCode::NOT_FOUND
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn returns_a_fallback_status_for_missing_or_undecodable_media() {
        let root = root("missing");
        let source = root.join("item.gif");
        fs::write(&source, b"GIF89a preview").unwrap();
        let library = Library::initialize(&root).unwrap();
        let item = imported(library.import_local_file(&source).unwrap());
        fs::remove_file(root.join("library").join("media").join(&item.storage_key)).unwrap();
        assert_eq!(
            respond(&library, &format!("/thumbnail/{}", item.id)).status(),
            StatusCode::NOT_FOUND
        );
        drop(library);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generates_bounded_static_png_thumbnails_and_reports_decode_failures() {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::new(4, 3));
        let mut source = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut source), ImageFormat::Png)
            .unwrap();

        let generated_thumbnail = thumbnail(&source, MediaFormat::Png).unwrap();
        assert_eq!(&generated_thumbnail[..8], b"\x89PNG\r\n\x1a\n");
        assert!(thumbnail(b"GIF89a invalid", MediaFormat::Gif).is_err());
        assert_eq!(media_type(MediaFormat::Gif), "image/gif");
        assert_eq!(media_type(MediaFormat::Webp), "image/webp");
        assert_eq!(media_type(MediaFormat::Png), "image/png");
    }
}
