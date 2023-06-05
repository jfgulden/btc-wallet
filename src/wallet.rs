use crate::{parser::BufferParser, error::CustomError};

#[derive(Clone, Debug)]
pub struct Wallet{
    pub name: String,
    pub pubkey: String,
    pub privkey: String,
}

impl Wallet{
    pub fn new(name: String, pubkey: String, privkey: String) -> Self{
        Self{
            name,
            pubkey,
            privkey,
        }
    }

    pub fn serialize(&self) -> Vec<u8>{
        let mut buffer = Vec::new();
        buffer.push(self.name.len() as u8);
        buffer.extend(self.name.as_bytes());
        buffer.push(self.pubkey.len() as u8);
        buffer.extend(self.pubkey.as_bytes());
        buffer.push(self.privkey.len() as u8);
        buffer.extend(self.privkey.as_bytes());
        buffer
    }

    pub fn parse_wallets(buffer: Vec<u8>) -> Result<Vec<Self>, CustomError> {
        let mut parser = BufferParser::new(buffer);
        let mut wallets = Vec::new();
        while parser.len() > 0{
            let name_len = parser.extract_u8()? as usize;
            let name = parser.extract_string(name_len)?;

            let pubkey_len = parser.extract_u8()? as usize;
            let pubkey = parser.extract_string(pubkey_len)?;

            let privkey_len = parser.extract_u8()? as usize;
            let privkey = parser.extract_string(privkey_len)?;

            println!("Wallet: {} {} {}", name, pubkey, privkey);
            wallets.push(Self::new(name, pubkey, privkey));
        }
        Ok(wallets)
    }
}