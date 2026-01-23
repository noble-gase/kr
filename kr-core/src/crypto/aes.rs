use anyhow::{anyhow, Result};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher, Crypter, Mode};

/// AES-CBC pkcs#7
pub struct CBC<K, I> {
    key: K,
    iv: I,
}

impl<K, I> CBC<K, I>
where
    K: AsRef<[u8]>,
    I: AsRef<[u8]>,
{
    pub fn new(key: K, iv: I) -> Self {
        Self { key, iv }
    }

    /// 填充字节, 默认: BlockSize(16)
    ///
    /// # Example
    ///
    /// ```
    /// let cbc = CBC::new(key, iv);
    ///
    /// // 默认填充
    /// let cipher = cbc.encrypt("plaintext", None).unwrap();
    ///
    /// // 指定填充字节
    /// let cipher = cbc.encrypt("plaintext", Some(32)).unwrap();
    /// ```
    pub fn encrypt(&self, data: impl AsRef<[u8]>, padding_size: Option<usize>) -> Result<Vec<u8>> {
        let t = self.cipher()?;
        let mut c = Crypter::new(t, Mode::Encrypt, self.key.as_ref(), Some(self.iv.as_ref()))?;
        c.pad(false);

        let v = pkcs7_padding(data.as_ref(), padding_size.unwrap_or(t.block_size()));
        let mut out = vec![0; v.len() + t.block_size()];
        let count = c.update(&v, &mut out)?;
        out.truncate(count);

        Ok(out)
    }

    /// # Example
    ///
    /// ```
    /// let cbc = CBC::new(key, iv);
    /// let plain = cbc.decrypt("ciphertext").unwrap();
    /// ```
    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<Vec<u8>> {
        let t = self.cipher()?;
        let mut c = Crypter::new(t, Mode::Decrypt, self.key.as_ref(), Some(self.iv.as_ref()))?;
        c.pad(false);

        let mut out = vec![0; data.as_ref().len() + t.block_size()];
        let count = c.update(data.as_ref(), &mut out)?;
        out.truncate(count);

        Ok(pkcs7_unpadding(&out))
    }

    fn cipher(&self) -> Result<Cipher> {
        let cipher = match self.key.as_ref().len() {
            16 => Cipher::aes_128_cbc(),
            24 => Cipher::aes_192_cbc(),
            32 => Cipher::aes_256_cbc(),
            _ => return Err(anyhow!("crypto/aes: invalid key size")),
        };
        Ok(cipher)
    }
}

/// AES-ECB pkcs#7
pub struct ECB<K> {
    key: K,
}

impl<K> ECB<K>
where
    K: AsRef<[u8]>,
{
    pub fn new(key: K) -> Self {
        Self { key }
    }

    /// 填充字节, 默认: BlockSize(16)
    ///
    /// # Example
    ///
    /// ```
    /// let ecb = ECB::new(key);
    ///
    /// // 默认填充
    /// let cipher = ecb.encrypt("plaintext", None).unwrap();
    ///
    /// // 指定填充字节
    /// let cipher = ecb.encrypt("plaintext", Some(32)).unwrap();
    /// ```
    pub fn encrypt(&self, data: impl AsRef<[u8]>, padding_size: Option<usize>) -> Result<Vec<u8>> {
        let t = self.cipher()?;
        let mut c = Crypter::new(t, Mode::Encrypt, self.key.as_ref(), None)?;
        c.pad(false);

        let v = pkcs7_padding(data.as_ref(), padding_size.unwrap_or(t.block_size()));
        let mut out = vec![0; v.len() + t.block_size()];
        let count = c.update(&v, &mut out)?;
        out.truncate(count);

        Ok(out)
    }

    /// # Example
    ///
    /// ```
    /// let ecb = ECB::new(key);
    /// let plain = ecb.decrypt("ciphertext").unwrap();
    /// ```
    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<Vec<u8>> {
        let t = self.cipher()?;
        let mut c = Crypter::new(t, Mode::Decrypt, self.key.as_ref(), None)?;
        c.pad(false);

        let mut out = vec![0; data.as_ref().len() + t.block_size()];
        let count = c.update(data.as_ref(), &mut out)?;
        out.truncate(count);

        Ok(pkcs7_unpadding(&out))
    }

    fn cipher(&self) -> Result<Cipher> {
        let cipher = match self.key.as_ref().len() {
            16 => Cipher::aes_128_ecb(),
            24 => Cipher::aes_192_ecb(),
            32 => Cipher::aes_256_ecb(),
            _ => return Err(anyhow!("crypto/aes: invalid key size")),
        };
        Ok(cipher)
    }
}

/// AES-GCM
pub struct GCM<K, N> {
    key: K,
    nonce: N,
}

impl<K, N> GCM<K, N>
where
    K: AsRef<[u8]>,
    N: AsRef<[u8]>,
{
    pub fn new(key: K, nonce: N) -> Self {
        Self { key, nonce }
    }

    /// [tag_size]: 默认=16, 可取范围 (12->16)
    ///
    /// # Example
    ///
    /// ```
    /// let gcm = GCM::new(key, nonce);
    ///
    /// // 默认 tag_size
    /// let (cipher, tag) = gcm.encrypt("plaintext", "aad", None).unwrap();
    ///
    /// // 指定 tag_size
    /// let (cipher, tag) = gcm.encrypt("plaintext", "aad", Some(12)).unwrap();
    /// ```
    pub fn encrypt(
        &self,
        data: impl AsRef<[u8]>,
        aad: impl AsRef<[u8]>,
        tag_size: Option<usize>,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        let t = self.cipher()?;
        let mut tag = vec![0; tag_size.unwrap_or(16)];
        let out = encrypt_aead(
            t,
            self.key.as_ref(),
            Some(self.nonce.as_ref()),
            aad.as_ref(),
            data.as_ref(),
            &mut tag,
        )?;
        Ok((out, tag))
    }

    /// # Example
    ///
    /// ```
    /// let gcm = GCM::new(key, nonce);
    /// let plain = gcm.decrypt("ciphertext", "aad", "tag").unwrap();
    /// ```
    pub fn decrypt(
        &self,
        data: impl AsRef<[u8]>,
        aad: impl AsRef<[u8]>,
        tag: impl AsRef<[u8]>,
    ) -> Result<Vec<u8>> {
        let t = self.cipher()?;
        let out = decrypt_aead(
            t,
            self.key.as_ref(),
            Some(self.nonce.as_ref()),
            aad.as_ref(),
            data.as_ref(),
            tag.as_ref(),
        )?;
        Ok(out)
    }

    fn cipher(&self) -> Result<Cipher> {
        let cipher = match self.key.as_ref().len() {
            16 => Cipher::aes_128_gcm(),
            24 => Cipher::aes_192_gcm(),
            32 => Cipher::aes_256_gcm(),
            _ => return Err(anyhow!("crypto/aes: invalid key size")),
        };
        Ok(cipher)
    }
}

fn pkcs7_padding(data: &[u8], block_size: usize) -> Vec<u8> {
    let mut padding = block_size - data.len() % block_size;
    if padding == 0 {
        padding = block_size
    }
    let mut b = [padding as u8; 1].repeat(padding);
    let mut v = data.to_vec();
    v.append(&mut b);
    v
}

fn pkcs7_unpadding(data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let padding = data[len - 1] as usize;
    data[..len - padding].to_vec()
}

#[cfg(test)]
mod tests {
    use base64::{prelude::BASE64_STANDARD, Engine};

    use crate::crypto::aes::{CBC, ECB, GCM};

    #[test]
    fn aes_cbc() {
        let key = "AES256Key-32Characters1234567890";
        let cbc = CBC::new(key, &key[..16]);

        // 默认填充
        let cipher = cbc.encrypt("ILoveRust", None).unwrap();
        assert_eq!(BASE64_STANDARD.encode(&cipher), "aXgPqNmb9UuorpPO/44xZA==");

        let plain = cbc.decrypt(&cipher).unwrap();
        assert_eq!(plain, b"ILoveRust");

        // 32字节填充
        let cipher2 = cbc.encrypt("ILoveRust", Some(32)).unwrap();
        assert_eq!(
            BASE64_STANDARD.encode(&cipher2),
            "6lj8Yn5eO5H9Sj2cEAe01MF+deF8VDokuCv6nLb9Cw4="
        );

        let plain2 = cbc.decrypt(&cipher2).unwrap();
        assert_eq!(plain2, b"ILoveRust");
    }

    #[test]
    fn aes_ecb() {
        let key = "AES256Key-32Characters1234567890";
        let ecb = ECB::new(key);

        // 默认填充
        let cipher = ecb.encrypt("ILoveRust", None).unwrap();
        assert_eq!(BASE64_STANDARD.encode(&cipher), "q0zwz5HYiN8b0h4mPaRCZw==");

        let plain = ecb.decrypt(&cipher).unwrap();
        assert_eq!(plain, b"ILoveRust");

        // 32字节填充
        let cipher2 = ecb.encrypt("ILoveRust", Some(32)).unwrap();
        assert_eq!(
            BASE64_STANDARD.encode(&cipher2),
            "3kcomMJ4/+z1CNQsuVKOqob5I9/o6GPWU0rcVuA+rn0="
        );

        let plain2 = ecb.decrypt(&cipher2).unwrap();
        assert_eq!(plain2, b"ILoveRust");
    }

    #[test]
    fn aes_gcm() {
        let key = "AES256Key-32Characters1234567890";
        let gcm = GCM::new(key, &key[..12]);

        // 默认 tag_size
        let (cipher, tag) = gcm.encrypt("ILoveRust", "IIInsomnia", None).unwrap();
        assert_eq!(BASE64_STANDARD.encode(&cipher), "qciumnRSNZQl");
        assert_eq!(BASE64_STANDARD.encode(&tag), "gQgezLrbimMH6tC7VsuyPg==");

        let plain = gcm.decrypt(&cipher, "IIInsomnia", &tag).unwrap();
        assert_eq!(plain, b"ILoveRust");

        // 指定 tag_size
        let (cipher2, tag2) = gcm.encrypt("ILoveRust", "IIInsomnia", Some(12)).unwrap();
        assert_eq!(BASE64_STANDARD.encode(&cipher2), "qciumnRSNZQl");
        assert_eq!(BASE64_STANDARD.encode(&tag2), "gQgezLrbimMH6tC7");

        let plain = gcm.decrypt(&cipher2, "IIInsomnia", &tag2).unwrap();
        assert_eq!(plain, b"ILoveRust");
    }
}
