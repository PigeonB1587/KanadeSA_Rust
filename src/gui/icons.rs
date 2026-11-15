//! Unity inventory fallback and optional white silhouettes, retaining source alpha.
use macroquad::prelude::*;
use std::collections::HashMap;

pub const FALLBACK: &str = crate::assets::FALLBACK_ICON;

pub fn path(icon: &str) -> String {
    let path = format!("{}/icons/{}.png", crate::assets::ROOT, icon.to_lowercase());
    if icon.is_empty() || !std::path::Path::new(&path).is_file() {
        FALLBACK.into()
    } else {
        path
    }
}

fn whiten(bytes: &mut [u8]) {
    for pixel in bytes.chunks_exact_mut(4) {
        pixel[..3].fill(255);
    }
}

#[derive(Default)]
pub struct IconCache {
    textures: HashMap<(String, bool), (Option<Texture2D>, u64)>,
    tick: u64,
    loads: usize,
}
impl IconCache {
    pub fn begin_frame(&mut self) {
        self.tick += 1;
        self.loads = 0;
    }
    pub fn get(&mut self, icon: &str, white: bool) -> Option<&Texture2D> {
        let key = (icon.to_owned(), white);
        if !self.textures.contains_key(&key) && self.loads < 2 {
            let load = |path: &str| -> Result<Image, String> {
                Image::from_file_with_format(
                    &std::fs::read(path).map_err(|e| e.to_string())?,
                    Some(ImageFormat::Png),
                )
                .map_err(|e| e.to_string())
            };
            let image = load(&path(icon)).or_else(|_| load(FALLBACK));
            let texture = match image {
                Ok(mut image) => {
                    if white {
                        whiten(&mut image.bytes);
                    }
                    let texture = Texture2D::from_image(&image);
                    texture.set_filter(FilterMode::Linear);
                    Some(texture)
                }
                Err(error) => {
                    tracing::warn!("Cannot load inventory fallback: {error}");
                    None
                }
            };
            if self.textures.len() >= 128 {
                if let Some(oldest) = self
                    .textures
                    .iter()
                    .min_by_key(|(_, (_, tick))| *tick)
                    .map(|(key, _)| key.clone())
                {
                    self.textures.remove(&oldest);
                }
            }
            self.textures.insert(key.clone(), (texture, self.tick));
            self.loads += 1;
        }
        self.textures.get_mut(&key).and_then(|(texture, tick)| {
            *tick = self.tick;
            texture.as_ref()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_empty_and_fake_paths_use_unity_rejection_letter() {
        for icon in ["", "fake/path", "UI/InventoryIcons/not_a_real_icon"] {
            assert_eq!(path(icon), FALLBACK);
        }
        assert_eq!(
            path("UI/InventoryIcons/emote-dance1"),
            "assets/icons/ui/inventoryicons/emote-dance1.png"
        );
        assert!(std::path::Path::new(FALLBACK).is_file());
    }

    #[test]
    fn white_icons_preserve_every_alpha_value() {
        let mut image = image::open(crate::assets::path(
            "icons/ui/inventoryicons/gun_ak_grey.png",
        ))
        .unwrap()
        .into_rgba8();
        let alpha: Vec<_> = image.pixels().map(|pixel| pixel[3]).collect();
        whiten(image.as_mut());
        for (pixel, alpha) in image.pixels().zip(alpha) {
            assert_eq!(pixel.0, [255, 255, 255, alpha]);
        }
    }
}
