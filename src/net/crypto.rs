//! AES-128 in CFB-8 mode, as used for the encrypted connection.

use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

pub struct Cfb8 {
    cipher: Aes128,
    reg: [u8; 16],
}

impl Cfb8 {
    /// The shared secret is both the key and the initial vector.
    pub fn new(secret: &[u8; 16]) -> Self {
        Cfb8 { cipher: Aes128::new(GenericArray::from_slice(secret)), reg: *secret }
    }

    #[inline]
    fn keystream(&self) -> u8 {
        let mut block = GenericArray::clone_from_slice(&self.reg);
        self.cipher.encrypt_block(&mut block);
        block[0]
    }

    #[inline]
    fn shift(&mut self, cipher_byte: u8) {
        self.reg.copy_within(1.., 0);
        self.reg[15] = cipher_byte;
    }

    pub fn encrypt(&mut self, data: &mut [u8]) {
        for b in data {
            *b ^= self.keystream();
            self.shift(*b);
        }
    }

    pub fn decrypt(&mut self, data: &mut [u8]) {
        for b in data {
            let c = *b;
            *b ^= self.keystream();
            self.shift(c);
        }
    }
}
