pub mod aes;
pub mod hash;

pub trait HashOutput {
    type Output;
    fn from_bytes(bytes: &[u8]) -> Self::Output;
}

impl HashOutput for Vec<u8> {
    type Output = Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self::Output {
        bytes.to_vec()
    }
}

impl HashOutput for String {
    type Output = String;
    fn from_bytes(bytes: &[u8]) -> Self::Output {
        const_hex::encode(bytes)
    }
}
