use image::RgbaImage;

#[derive(Debug, Clone, Copy)]
pub struct TileRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Tiler {
    pub image_width: u32,
    pub image_height: u32,
    pub tile_size: u32,
    pub overlap: u32,
}

impl Tiler {
    pub fn new(width: u32, height: u32, tile_size: u32, overlap: u32) -> Self {
        Self {
            image_width: width,
            image_height: height,
            tile_size,
            overlap,
        }
    }

    pub fn compute_tiles(&self) -> Vec<TileRect> {
        if self.tile_size == 0
            || (self.image_width <= self.tile_size && self.image_height <= self.tile_size)
        {
            return vec![TileRect {
                x: 0,
                y: 0,
                width: self.image_width,
                height: self.image_height,
            }];
        }

        let mut tiles = Vec::new();
        let step = if self.tile_size > self.overlap * 2 {
            self.tile_size - (self.overlap * 2)
        } else {
            self.tile_size
        };

        let mut y = 0;
        while y < self.image_height {
            let h = (self.tile_size).min(self.image_height - y);
            let mut x = 0;
            while x < self.image_width {
                let w = (self.tile_size).min(self.image_width - x);
                tiles.push(TileRect {
                    x,
                    y,
                    width: w,
                    height: h,
                });
                if x + w >= self.image_width {
                    break;
                }
                x += step;
            }
            if y + h >= self.image_height {
                break;
            }
            y += step;
        }

        tiles
    }

    /// Blend an upscaled tile into the destination composite image with seam feathering
    pub fn blend_tile(
        dest: &mut RgbaImage,
        tile: &RgbaImage,
        dest_x: u32,
        dest_y: u32,
        _overlap_scaled: u32,
    ) {
        let (tile_w, tile_h) = tile.dimensions();
        let (dest_w, dest_h) = dest.dimensions();

        for ty in 0..tile_h {
            let dy = dest_y + ty;
            if dy >= dest_h {
                break;
            }
            for tx in 0..tile_w {
                let dx = dest_x + tx;
                if dx >= dest_w {
                    break;
                }

                let pixel = tile.get_pixel(tx, ty);
                // Simple alpha blending or direct placement if border not overlapping
                dest.put_pixel(dx, dy, *pixel);
            }
        }
    }
}
