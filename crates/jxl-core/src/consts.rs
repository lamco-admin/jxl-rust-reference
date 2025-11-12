//! Constants used throughout JPEG XL implementation

/// Maximum supported image dimension
pub const MAX_IMAGE_DIMENSION: u32 = 268435456; // 2^28

/// Maximum number of frames in an animation
pub const MAX_NUM_FRAMES: u32 = 2147483647; // 2^31 - 1

/// Block sizes
pub const BLOCK_SIZE: usize = 8;
pub const GROUP_SIZE: usize = 256;
pub const DC_GROUP_SIZE: usize = 2048;

/// Number of DC groups per AC group
pub const DC_GROUPS_PER_AC_GROUP: usize = DC_GROUP_SIZE / GROUP_SIZE;

/// Maximum number of color channels
pub const MAX_CHANNELS: usize = 4;

/// Default quality for lossy encoding (0-100)
pub const DEFAULT_QUALITY: f32 = 90.0;

/// Default encoding effort (1-9)
pub const DEFAULT_EFFORT: u8 = 7;

/// Minimum and maximum quality values
pub const MIN_QUALITY: f32 = 0.0;
pub const MAX_QUALITY: f32 = 100.0;

/// Minimum and maximum effort values
pub const MIN_EFFORT: u8 = 1;
pub const MAX_EFFORT: u8 = 9;
