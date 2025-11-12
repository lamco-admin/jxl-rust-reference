//! JPEG XL container format (ISO/IEC 18181-2)
//!
//! JPEG XL supports two bitstream formats:
//! 1. Naked codestream: 0xFF0A signature (minimal overhead)
//! 2. Container format: Box-based structure (recommended for files)
//!
//! This module implements the container format with ISOBMFF-style boxes.

use jxl_core::*;
use std::io::{Read, Write};

/// JPEG XL container signature (12 bytes)
///
/// Format: `\0\0\0\x0C JXL \x0D\x0A\x87\x0A`
/// - First 4 bytes: Box size (12 for signature box)
/// - Next 4 bytes: "JXL " (box type)
/// - Last 4 bytes: CR+LF+0x87+LF (corruption detection)
pub const CONTAINER_SIGNATURE: [u8; 12] = [
    0x00, 0x00, 0x00, 0x0C, // Box size = 12
    0x4A, 0x58, 0x4C, 0x20, // "JXL "
    0x0D, 0x0A, 0x87, 0x0A, // CR LF 0x87 LF
];

/// Naked codestream signature (2 bytes)
///
/// Format: `0xFF 0x0A`
pub const CODESTREAM_SIGNATURE: [u8; 2] = [0xFF, 0x0A];

/// File type box (ftyp) brand
pub const BRAND_JXL: [u8; 4] = [0x6A, 0x78, 0x6C, 0x20]; // "jxl "

/// Box types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxType {
    /// File type box
    FileType,
    /// JXL codestream box
    JxlCodestream,
    /// Partial JXL codestream box
    JxlPartial,
    /// Exif metadata
    Exif,
    /// XML metadata
    Xml,
    /// JSON metadata
    Json,
    /// Unknown/custom box
    Unknown([u8; 4]),
}

impl BoxType {
    pub fn from_fourcc(fourcc: &[u8; 4]) -> Self {
        match fourcc {
            b"ftyp" => BoxType::FileType,
            b"jxlc" => BoxType::JxlCodestream,
            b"jxlp" => BoxType::JxlPartial,
            b"Exif" => BoxType::Exif,
            b"xml " => BoxType::Xml,
            b"json" => BoxType::Json,
            _ => BoxType::Unknown(*fourcc),
        }
    }

    pub fn to_fourcc(&self) -> [u8; 4] {
        match self {
            BoxType::FileType => *b"ftyp",
            BoxType::JxlCodestream => *b"jxlc",
            BoxType::JxlPartial => *b"jxlp",
            BoxType::Exif => *b"Exif",
            BoxType::Xml => *b"xml ",
            BoxType::Json => *b"json",
            BoxType::Unknown(fourcc) => *fourcc,
        }
    }
}

/// A box in the JPEG XL container
#[derive(Debug, Clone)]
pub struct JxlBox {
    pub box_type: BoxType,
    pub data: Vec<u8>,
}

impl JxlBox {
    pub fn new(box_type: BoxType, data: Vec<u8>) -> Self {
        Self { box_type, data }
    }

    /// Create a file type box
    pub fn file_type(brand: [u8; 4], minor_version: u32, compatible_brands: Vec<[u8; 4]>) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&brand);
        data.extend_from_slice(&minor_version.to_be_bytes());
        for compat_brand in compatible_brands {
            data.extend_from_slice(&compat_brand);
        }
        Self::new(BoxType::FileType, data)
    }

    /// Create a JXL codestream box
    pub fn jxl_codestream(codestream_data: Vec<u8>) -> Self {
        Self::new(BoxType::JxlCodestream, codestream_data)
    }

    /// Write box to output
    pub fn write<W: Write>(&self, writer: &mut W) -> JxlResult<()> {
        // Calculate total box size (8 bytes header + data length)
        let box_size = 8 + self.data.len() as u64;

        // Write box size (big-endian u32, or 1 for extended size)
        if box_size <= u32::MAX as u64 {
            writer.write_all(&(box_size as u32).to_be_bytes())?;
        } else {
            // Extended box size (not common, but spec-compliant)
            writer.write_all(&1u32.to_be_bytes())?;
        }

        // Write box type (fourcc)
        writer.write_all(&self.box_type.to_fourcc())?;

        // Write extended size if needed
        if box_size > u32::MAX as u64 {
            writer.write_all(&box_size.to_be_bytes())?;
        }

        // Write box data
        writer.write_all(&self.data)?;

        Ok(())
    }

    /// Read box from input
    pub fn read<R: Read>(reader: &mut R) -> JxlResult<Self> {
        // Read box size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let mut box_size = u32::from_be_bytes(size_bytes) as u64;

        // Read box type
        let mut type_bytes = [0u8; 4];
        reader.read_exact(&mut type_bytes)?;
        let box_type = BoxType::from_fourcc(&type_bytes);

        // Handle extended size
        if box_size == 1 {
            let mut extended_size_bytes = [0u8; 8];
            reader.read_exact(&mut extended_size_bytes)?;
            box_size = u64::from_be_bytes(extended_size_bytes);
        }

        // Read box data (size - header bytes)
        let header_size = if box_size == 1 { 16 } else { 8 };
        let data_size = (box_size - header_size) as usize;
        let mut data = vec![0u8; data_size];
        reader.read_exact(&mut data)?;

        Ok(Self { box_type, data })
    }
}

/// JPEG XL container
#[derive(Debug, Clone)]
pub struct Container {
    pub boxes: Vec<JxlBox>,
}

impl Container {
    pub fn new() -> Self {
        Self { boxes: Vec::new() }
    }

    /// Create a container with default boxes for a single codestream
    pub fn with_codestream(codestream_data: Vec<u8>) -> Self {
        let mut container = Self::new();

        // Add file type box
        container.boxes.push(JxlBox::file_type(
            BRAND_JXL,
            0, // Minor version
            vec![BRAND_JXL],
        ));

        // Add codestream box
        container.boxes.push(JxlBox::jxl_codestream(codestream_data));

        container
    }

    /// Write container to output
    pub fn write<W: Write>(&self, writer: &mut W) -> JxlResult<()> {
        // Write container signature
        writer.write_all(&CONTAINER_SIGNATURE)?;

        // Write all boxes
        for box_item in &self.boxes {
            box_item.write(writer)?;
        }

        Ok(())
    }

    /// Read container from input
    pub fn read<R: Read>(reader: &mut R) -> JxlResult<Self> {
        // Read and verify signature
        let mut signature = [0u8; 12];
        reader.read_exact(&mut signature)?;

        if signature != CONTAINER_SIGNATURE {
            return Err(JxlError::InvalidSignature);
        }

        // Read all boxes
        let mut boxes = Vec::new();
        loop {
            match JxlBox::read(reader) {
                Ok(box_item) => boxes.push(box_item),
                Err(JxlError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Self { boxes })
    }

    /// Extract codestream data from container
    pub fn extract_codestream(&self) -> JxlResult<Vec<u8>> {
        let mut codestream = Vec::new();

        for box_item in &self.boxes {
            match box_item.box_type {
                BoxType::JxlCodestream => {
                    codestream.extend_from_slice(&box_item.data);
                }
                BoxType::JxlPartial => {
                    // Partial codestream boxes are concatenated
                    codestream.extend_from_slice(&box_item.data);
                }
                _ => {} // Ignore other boxes
            }
        }

        if codestream.is_empty() {
            return Err(JxlError::InvalidBitstream(
                "No codestream found in container".to_string(),
            ));
        }

        Ok(codestream)
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_signature() {
        assert_eq!(CONTAINER_SIGNATURE.len(), 12);
        assert_eq!(&CONTAINER_SIGNATURE[4..8], b"JXL ");
    }

    #[test]
    fn test_box_type_conversion() {
        assert_eq!(BoxType::from_fourcc(b"ftyp"), BoxType::FileType);
        assert_eq!(BoxType::from_fourcc(b"jxlc"), BoxType::JxlCodestream);
        assert_eq!(BoxType::FileType.to_fourcc(), *b"ftyp");
    }

    #[test]
    fn test_file_type_box() {
        let ftyp = JxlBox::file_type(BRAND_JXL, 0, vec![BRAND_JXL]);
        assert_eq!(ftyp.box_type, BoxType::FileType);
        assert_eq!(&ftyp.data[0..4], b"jxl ");
    }

    #[test]
    fn test_container_roundtrip() {
        let codestream = vec![0xFF, 0x0A, 0x00, 0x01, 0x02, 0x03];
        let container = Container::with_codestream(codestream.clone());

        let mut buffer = Vec::new();
        container.write(&mut buffer).unwrap();

        let parsed = Container::read(&mut buffer.as_slice()).unwrap();
        let extracted = parsed.extract_codestream().unwrap();

        assert_eq!(extracted, codestream);
    }
}
