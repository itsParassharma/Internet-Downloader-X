// SPDX-License-Identifier: MIT
//! Decoded image resources from the user-supplied application and toolbar skin.
//! Image provenance and asset rights are recorded in assets/idm/ASSET-NOTICE.txt.
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;

const TOOLBAR_WIDTH: usize = 672;
const TOOLBAR_HEIGHT: usize = 42;
const TOOLBAR_TILE_WIDTH: usize = 56;
const TOOLBAR_COUNT: usize = 12;
const TOOLBAR_NORMAL: &[u8] = include_bytes!("../assets/idm/toolbar_normal.rgba");
const TOOLBAR_HOT: &[u8] = include_bytes!("../assets/idm/toolbar_hot.rgba");

pub struct UiAssets {
    pub toolbar: Vec<TextureHandle>,
    pub toolbar_hot: Vec<TextureHandle>,
    pub toolbar_disabled: Vec<TextureHandle>,
    pub sidebar: HashMap<&'static str, TextureHandle>,
}

impl UiAssets {
    pub fn new(ctx: &egui::Context) -> Self {
        let toolbar = load_toolbar(ctx, "classic-normal", TOOLBAR_NORMAL, false);
        let toolbar_hot = load_toolbar(ctx, "classic-hover", TOOLBAR_HOT, false);
        let toolbar_disabled = load_toolbar(ctx, "classic-disabled", TOOLBAR_NORMAL, true);
        let entries: [(&'static str, &[u8]); 11] = [
            ("all", include_bytes!("../assets/idm/sidebar_all.rgba")),
            (
                "unfinished",
                include_bytes!("../assets/idm/sidebar_unfinished.rgba"),
            ),
            (
                "finished",
                include_bytes!("../assets/idm/sidebar_finished.rgba"),
            ),
            (
                "compressed",
                include_bytes!("../assets/idm/sidebar_compressed.rgba"),
            ),
            (
                "documents",
                include_bytes!("../assets/idm/sidebar_documents.rgba"),
            ),
            ("music", include_bytes!("../assets/idm/sidebar_music.rgba")),
            (
                "programs",
                include_bytes!("../assets/idm/sidebar_programs.rgba"),
            ),
            ("video", include_bytes!("../assets/idm/sidebar_video.rgba")),
            ("other", include_bytes!("../assets/idm/sidebar_other.rgba")),
            (
                "queues",
                include_bytes!("../assets/idm/sidebar_queues.rgba"),
            ),
            (
                "grabber",
                include_bytes!("../assets/idm/sidebar_grabber.rgba"),
            ),
        ];
        let sidebar = entries
            .into_iter()
            .map(|(key, rgba)| {
                assert_eq!(
                    rgba.len(),
                    16 * 13 * 4,
                    "Incorrect sidebar image size for {key}"
                );
                let image = ColorImage::from_rgba_unmultiplied([16, 13], rgba);
                (
                    key,
                    ctx.load_texture(format!("sidebar-{key}"), image, TextureOptions::LINEAR),
                )
            })
            .collect();
        Self {
            toolbar,
            toolbar_hot,
            toolbar_disabled,
            sidebar,
        }
    }

    /// Add, Resume, Stop, Stop All, Delete, Delete Completed, Options,
    /// Scheduler, Start Queue, Stop Queue, Grabber, Tell a Friend.
    pub fn toolbar(&self, index: usize, hot: bool) -> &TextureHandle {
        let textures = if hot {
            &self.toolbar_hot
        } else {
            &self.toolbar
        };
        &textures[index.min(TOOLBAR_COUNT - 1)]
    }

    pub fn toolbar_disabled(&self, index: usize) -> &TextureHandle {
        &self.toolbar_disabled[index.min(TOOLBAR_COUNT - 1)]
    }

    pub fn sidebar(&self, key: &str) -> &TextureHandle {
        self.sidebar
            .get(key)
            .unwrap_or_else(|| &self.sidebar["other"])
    }
}

fn load_toolbar(
    ctx: &egui::Context,
    name: &str,
    source: &[u8],
    disabled: bool,
) -> Vec<TextureHandle> {
    assert_eq!(source.len(), TOOLBAR_WIDTH * TOOLBAR_HEIGHT * 4);
    (0..TOOLBAR_COUNT)
        .map(|index| {
            let mut rgba = Vec::with_capacity(TOOLBAR_TILE_WIDTH * TOOLBAR_HEIGHT * 4);
            for y in 0..TOOLBAR_HEIGHT {
                let start = (y * TOOLBAR_WIDTH + index * TOOLBAR_TILE_WIDTH) * 4;
                rgba.extend_from_slice(&source[start..start + TOOLBAR_TILE_WIDTH * 4]);
            }
            if disabled {
                // The supplied classic skin has normal and hover strips only.
                for pixel in rgba.chunks_exact_mut(4) {
                    let gray = ((77 * pixel[0] as u16
                        + 150 * pixel[1] as u16
                        + 29 * pixel[2] as u16)
                        >> 8) as u8;
                    pixel[0] = gray;
                    pixel[1] = gray;
                    pixel[2] = gray;
                    pixel[3] = (pixel[3] as u16 * 145 / 255) as u8;
                }
            }
            let image =
                ColorImage::from_rgba_unmultiplied([TOOLBAR_TILE_WIDTH, TOOLBAR_HEIGHT], &rgba);
            ctx.load_texture(
                format!("toolbar-{name}-{index}"),
                image,
                TextureOptions::LINEAR,
            )
        })
        .collect()
}

/// Original IDX X-and-download-arrow logo, distinct from the supplied toolbar artwork.
#[cfg(not(windows))]
pub fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/idx-main.rgba").to_vec(),
        width: 256,
        height: 256,
    }
}
