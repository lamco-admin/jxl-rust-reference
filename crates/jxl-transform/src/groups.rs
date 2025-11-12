//! Group processing for JPEG XL
//!
//! JPEG XL divides images into groups for parallel processing:
//! - DC groups: 2048×2048 pixel regions (256×256 blocks)
//! - AC groups: 256×256 pixel regions (32×32 blocks)

use jxl_core::{Dimensions, JxlResult};

/// Size of a block in pixels (8x8)
pub const BLOCK_SIZE: usize = 8;

/// Size of an AC group in pixels (256x256)
pub const AC_GROUP_SIZE: usize = 256;

/// Size of a DC group in pixels (2048x2048)
pub const DC_GROUP_SIZE: usize = 2048;

/// Size of an AC group in blocks (32x32)
pub const AC_GROUP_SIZE_IN_BLOCKS: usize = AC_GROUP_SIZE / BLOCK_SIZE;

/// Size of a DC group in blocks (256x256)
pub const DC_GROUP_SIZE_IN_BLOCKS: usize = DC_GROUP_SIZE / BLOCK_SIZE;

/// Represents a group of DCT coefficients
#[derive(Debug, Clone)]
pub struct Group {
    /// Group position (in group coordinates)
    pub x: usize,
    pub y: usize,
    /// Group size in pixels
    pub width: usize,
    pub height: usize,
    /// DCT coefficients for this group (stored per-channel)
    pub coefficients: Vec<Vec<i16>>,
}

impl Group {
    /// Create a new group
    pub fn new(x: usize, y: usize, width: usize, height: usize, num_channels: usize) -> Self {
        let size = width * height;
        let mut coefficients = Vec::new();
        for _ in 0..num_channels {
            coefficients.push(vec![0; size]);
        }

        Self {
            x,
            y,
            width,
            height,
            coefficients,
        }
    }
}

/// Calculate the number of groups needed for a dimension
pub fn num_groups(size: usize, group_size: usize) -> usize {
    size.div_ceil(group_size)
}

/// Calculate group dimensions
pub fn calculate_group_dimensions(
    image_dims: Dimensions,
    group_size: usize,
) -> JxlResult<(usize, usize)> {
    let num_groups_x = num_groups(image_dims.width as usize, group_size);
    let num_groups_y = num_groups(image_dims.height as usize, group_size);

    Ok((num_groups_x, num_groups_y))
}

/// Get the actual size of a group (accounting for edge groups)
pub fn get_group_size(
    group_x: usize,
    group_y: usize,
    image_dims: Dimensions,
    group_size: usize,
) -> (usize, usize) {
    let start_x = group_x * group_size;
    let start_y = group_y * group_size;

    let width = group_size.min(image_dims.width as usize - start_x);
    let height = group_size.min(image_dims.height as usize - start_y);

    (width, height)
}

/// Divide an image into groups for processing
pub fn create_groups(
    image_dims: Dimensions,
    num_channels: usize,
    group_size: usize,
) -> JxlResult<Vec<Group>> {
    let (num_groups_x, num_groups_y) = calculate_group_dimensions(image_dims, group_size)?;
    let mut groups = Vec::new();

    for gy in 0..num_groups_y {
        for gx in 0..num_groups_x {
            let (width, height) = get_group_size(gx, gy, image_dims, group_size);
            groups.push(Group::new(gx, gy, width, height, num_channels));
        }
    }

    Ok(groups)
}

/// Extract a group's pixel data from an image buffer
pub fn extract_group_pixels(
    image_buffer: &[f32],
    image_width: usize,
    image_height: usize,
    group_x: usize,
    group_y: usize,
    group_size: usize,
) -> Vec<f32> {
    let start_x = group_x * group_size;
    let start_y = group_y * group_size;

    let width = group_size.min(image_width - start_x);
    let height = group_size.min(image_height - start_y);

    let mut group_pixels = vec![0.0; width * height];

    for y in 0..height {
        for x in 0..width {
            let src_idx = (start_y + y) * image_width + (start_x + x);
            let dst_idx = y * width + x;
            group_pixels[dst_idx] = image_buffer[src_idx];
        }
    }

    group_pixels
}

/// Insert a group's pixel data back into an image buffer
pub fn insert_group_pixels(
    group_pixels: &[f32],
    image_buffer: &mut [f32],
    image_width: usize,
    image_height: usize,
    group_x: usize,
    group_y: usize,
    group_size: usize,
) {
    let start_x = group_x * group_size;
    let start_y = group_y * group_size;

    let width = group_size.min(image_width - start_x);
    let height = group_size.min(image_height - start_y);

    for y in 0..height {
        for x in 0..width {
            let src_idx = y * width + x;
            let dst_idx = (start_y + y) * image_width + (start_x + x);
            image_buffer[dst_idx] = group_pixels[src_idx];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_groups() {
        assert_eq!(num_groups(256, 256), 1);
        assert_eq!(num_groups(512, 256), 2);
        assert_eq!(num_groups(300, 256), 2);
        assert_eq!(num_groups(2048, 256), 8);
    }

    #[test]
    fn test_create_groups() {
        let dims = Dimensions::new(512, 512);
        let groups = create_groups(dims, 3, AC_GROUP_SIZE).unwrap();
        assert_eq!(groups.len(), 4); // 2x2 grid

        for group in groups {
            assert!(group.width <= AC_GROUP_SIZE);
            assert!(group.height <= AC_GROUP_SIZE);
            assert_eq!(group.coefficients.len(), 3); // 3 channels
        }
    }

    #[test]
    fn test_extract_insert_group() {
        let image_width = 512;
        let image_height = 512;
        let mut image = vec![0.0; image_width * image_height];

        // Fill with test pattern
        for y in 0..image_height {
            for x in 0..image_width {
                image[y * image_width + x] = (x + y) as f32;
            }
        }

        // Extract a group
        let group_pixels = extract_group_pixels(&image, image_width, image_height, 1, 1, 256);
        assert_eq!(group_pixels.len(), 256 * 256);

        // Verify first pixel matches
        assert_eq!(group_pixels[0], image[256 * image_width + 256]);

        // Clear image and insert group back
        let mut new_image = vec![0.0; image_width * image_height];
        insert_group_pixels(
            &group_pixels,
            &mut new_image,
            image_width,
            image_height,
            1,
            1,
            256,
        );

        // Verify the group region matches
        for y in 256..512 {
            for x in 256..512 {
                assert_eq!(new_image[y * image_width + x], image[y * image_width + x]);
            }
        }
    }
}
