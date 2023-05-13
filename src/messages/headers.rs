use bitcoin_hashes::{sha256d, Hash};

use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
};

#[derive(Debug)]

pub struct Headers {
    pub headers: Vec<BlockHeader>,
}

#[derive(Debug)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block_hash: Vec<u8>,
    pub merkle_root: Vec<u8>,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.version.to_le_bytes());
        buffer.extend(&self.prev_block_hash);
        buffer.extend(&self.merkle_root);
        buffer.extend(&self.timestamp.to_le_bytes());
        buffer.extend(&self.bits.to_le_bytes());
        buffer.extend(&self.nonce.to_le_bytes());
        buffer
    }
    pub fn parse(buffer: Vec<u8>, validate: bool) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        if parser.len() < 80 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let block_header = BlockHeader {
            version: parser.extract_i32()?,
            prev_block_hash: parser.extract_buffer(32)?.to_vec(),
            merkle_root: parser.extract_buffer(32)?.to_vec(),
            timestamp: parser.extract_u32()?,
            bits: parser.extract_u32()?,
            nonce: parser.extract_u32()?,
        };

        if validate {
            let is_valid = block_header.validate();

            if !is_valid {
                return Err(CustomError::HeaderInvalidPoW);
            }
        }

        Ok(block_header)
    }

    fn validate(&self) -> bool {
        let hash = self.hash();
        let bits_vec = self.bits.to_be_bytes().to_vec();

        let leading_zeros_start = *bits_vec.get(0).unwrap() as usize;
        let leading_zeros = hash.get(leading_zeros_start..32).unwrap().to_vec();

        if leading_zeros.iter().find(|zero| **zero != 0_u8).is_some() {
            return false;
        }

        let mut significants = hash
            .get((leading_zeros_start - 3)..leading_zeros_start)
            .unwrap()
            .to_vec();
        significants.reverse();

        let mut bits_vec_pos = 1;
        for hash_byte in significants {
            if hash_byte != bits_vec[bits_vec_pos] {
                return hash_byte < bits_vec[bits_vec_pos];
            }
            bits_vec_pos += 1;
        }

        return false;
    }

    pub fn hash(&self) -> Vec<u8> {
        sha256d::Hash::hash(&self.serialize())
            .to_byte_array()
            .to_vec()
    }
}

impl Headers {
    pub fn new() -> Self {
        Headers { headers: vec![] }
    }
    pub fn serialize_headers(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        for header in &self.headers {
            let header_buffer: Vec<u8> = header.serialize();
            buffer.extend(header_buffer);
        }
        buffer
    }
    pub fn parse_headers(buffer: Vec<u8>) -> Result<Vec<BlockHeader>, CustomError> {
        let mut parser = BufferParser::new(buffer);
        if parser.len() % 80 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let mut headers = vec![];
        while !parser.is_empty() {
            headers.push(BlockHeader::parse(
                parser.extract_buffer(80)?.to_vec(),
                false,
            )?);
        }
        Ok(headers)
    }
}

impl Message for Headers {
    fn get_command(&self) -> String {
        String::from("headers")
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.headers.len().to_varint_bytes());
        for header in &self.headers {
            buffer.extend(&header.serialize());
            buffer.extend(0_u8.to_le_bytes());
        }
        buffer
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        println!("buffer: {:?}", buffer);
        let mut parser = BufferParser::new(buffer);

        let header_count = parser.extract_varint()?;
        if parser.len() % 81 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let mut headers = vec![];
        while parser.len() >= 81 {
            headers.push(BlockHeader::parse(
                parser.extract_buffer(81)?.to_vec(),
                true,
            )?);
        }

        println!("header count: {}", header_count);
        Ok(Headers { headers })
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn header_serialize_and_parse() {
        let buffer = vec![
            1, 0, 0, 0, 5, 159, 141, 74, 195, 4, 19, 253, 127, 1, 148, 149, 222, 143, 237, 24, 27,
            124, 186, 34, 123, 241, 216, 166, 203, 239, 86, 108, 0, 0, 0, 0, 233, 233, 109, 115,
            249, 241, 6, 200, 176, 73, 10, 24, 28, 209, 102, 159, 255, 179, 239, 72, 185, 225, 10,
            14, 219, 74, 174, 208, 207, 59, 18, 12, 170, 7, 195, 79, 255, 255, 0, 29, 14, 171, 58,
            61,
        ];

        let buffer_clone = buffer.clone();

        let block_header = BlockHeader::parse(buffer, true).unwrap();
        let serialized_block_header = block_header.serialize();

        assert_eq!(buffer_clone, serialized_block_header);
    }

    #[test]
    fn headers_invalid_buffer() {
        let buffer = vec![
            1, 0, 0, 0, 5, 159, 141, 74, 195, 4, 19, 253, 127, 1, 148, 149, 222, 143, 237, 24, 27,
            124, 186, 34, 123, 241, 216, 166, 203, 239, 86, 108, 0, 0, 0, 0, 233, 233, 109, 115,
            249, 241, 6, 200, 176, 73, 10, 24, 28, 209, 102, 159, 255, 179, 239, 72, 185, 225, 10,
            14, 219,
        ];

        let block_header = BlockHeader::parse(buffer, true);

        assert!(block_header.is_err());
    }

    #[test]
    fn valid_pow_header() {
        let valid_header = BlockHeader {
            version: 2,
            prev_block_hash: vec![
                61, 8, 52, 163, 234, 98, 255, 92, 186, 170, 164, 90, 56, 131, 46, 171, 52, 239,
                104, 223, 166, 65, 183, 217, 36, 6, 53, 63, 0, 0, 0, 0,
            ],
            merkle_root: vec![
                45, 107, 6, 225, 181, 124, 4, 88, 86, 174, 58, 59, 113, 215, 174, 42, 209, 149,
                142, 110, 166, 53, 244, 88, 6, 76, 228, 77, 7, 10, 189, 126,
            ],
            timestamp: 1347149007,
            bits: 476726600,
            nonce: 240236131,
        };

        assert!(valid_header.validate());
    }

    #[test]
    fn invalid_pow_header() {
        let valid_header = BlockHeader {
            version: 2,
            prev_block_hash: vec![
                61, 8, 52, 163, 234, 98, 255, 92, 186, 170, 164, 90, 56, 131, 46, 171, 52, 239,
                104, 223, 166, 65, 183, 217, 36, 6, 53, 63, 0, 0, 0, 0,
            ],
            merkle_root: vec![
                45, 107, 6, 225, 181, 124, 4, 88, 86, 174, 58, 59, 113, 215, 174, 42, 209, 149,
                142, 110, 166, 53, 244, 88, 6, 76, 228, 77, 7, 10, 189, 126,
            ],
            timestamp: 1347149007,
            bits: 476726600,
            nonce: 123123,
        };

        assert!(!valid_header.validate());
    }

    #[test]
    fn parse_and_serialize_headers() {
        let buffer = vec![
            1, 0, 0, 128, 32, 169, 255, 173, 21, 40, 44, 123, 115, 129, 193, 143, 57, 71, 116, 199,
            75, 244, 113, 169, 45, 227, 42, 180, 111, 0, 0, 0, 0, 0, 0, 0, 0, 109, 105, 250, 106,
            92, 126, 17, 171, 97, 243, 124, 194, 172, 252, 249, 166, 202, 8, 231, 136, 21, 107,
            106, 136, 64, 241, 195, 82, 179, 236, 159, 63, 155, 22, 96, 100, 105, 90, 32, 25, 11,
            42, 241, 166, 0,
        ];

        let buffer_clone = buffer.clone();

        let headers = Headers::parse(buffer).unwrap();
        let serialized_headers = headers.serialize();

        assert_eq!(buffer_clone, serialized_headers);
    }

    #[test]
    fn invalid_header() {
        let buffer = vec![
            1, 0, 0, 128, 32, 169, 255, 173, 21, 40, 44, 123, 115, 129, 193, 143, 57, 71, 116, 199,
            75, 244, 113, 169, 45, 227, 42, 180, 111, 0, 0, 0, 0, 0, 0, 0, 0, 109, 105, 250, 106,
            92, 126, 17, 171, 9,
        ];

        let headers = Headers::parse(buffer);

        assert!(headers.is_err());
    }
}