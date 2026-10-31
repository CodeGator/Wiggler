use eframe::egui::IconData;
use tray_icon::Icon;

use crate::theme::{AMBER, GREEN};

pub fn window_icon() -> IconData {
    let (rgba, width, height) = orb_rgba(256, GREEN.to_array());
    IconData {
        rgba,
        width,
        height,
    }
}

pub fn tray_icon(running: bool) -> Icon {
    let fill = if running { GREEN } else { AMBER };
    let (rgba, width, height) = orb_rgba(32, fill.to_array());
    Icon::from_rgba(rgba, width, height).expect("tray icon")
}

fn orb_rgba(size: u32, fill: [u8; 4]) -> (Vec<u8>, u32, u32) {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    let cx = (size as f32 - 1.0) * 0.5;
    let cy = cx;
    let radius = size as f32 * 0.38;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            let alpha = if dist <= radius - 1.0 {
                1.0
            } else if dist >= radius + 1.0 {
                0.0
            } else {
                ((radius + 1.0) - dist) * 0.5
            };
            if alpha <= 0.0 {
                continue;
            }
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = fill[0];
            rgba[i + 1] = fill[1];
            rgba[i + 2] = fill[2];
            rgba[i + 3] = (f32::from(fill[3]) * alpha) as u8;
        }
    }
    (rgba, size, size)
}
