//! Reading and writing protocol data types (std only).

use std::fmt;

#[derive(Debug)]
pub struct DecodeError(pub String);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decode error: {}", self.0)
    }
}
impl std::error::Error for DecodeError {}

pub type DResult<T> = Result<T, DecodeError>;

fn eof<T>() -> DResult<T> {
    Err(DecodeError("unexpected end of packet".into()))
}

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    pub fn bytes(&mut self, n: usize) -> DResult<&'a [u8]> {
        if self.remaining() < n {
            return eof();
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    pub fn rest(&mut self) -> &'a [u8] {
        let s = &self.data[self.pos..];
        self.pos = self.data.len();
        s
    }
    pub fn u8(&mut self) -> DResult<u8> {
        Ok(self.bytes(1)?[0])
    }
    pub fn bool(&mut self) -> DResult<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn i8(&mut self) -> DResult<i8> {
        Ok(self.u8()? as i8)
    }
    pub fn u16(&mut self) -> DResult<u16> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    pub fn i16(&mut self) -> DResult<i16> {
        Ok(self.u16()? as i16)
    }
    pub fn i32(&mut self) -> DResult<i32> {
        Ok(i32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    pub fn i64(&mut self) -> DResult<i64> {
        Ok(i64::from_be_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> DResult<f32> {
        Ok(f32::from_bits(self.i32()? as u32))
    }
    pub fn f64(&mut self) -> DResult<f64> {
        Ok(f64::from_bits(self.i64()? as u64))
    }
    pub fn varint(&mut self) -> DResult<i32> {
        let mut v: u32 = 0;
        for i in 0..5 {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u32) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(v as i32);
            }
        }
        Err(DecodeError("varint too long".into()))
    }
    pub fn varlong(&mut self) -> DResult<i64> {
        let mut v: u64 = 0;
        for i in 0..10 {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u64) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(v as i64);
            }
        }
        Err(DecodeError("varlong too long".into()))
    }
    pub fn len(&mut self) -> DResult<usize> {
        let n = self.varint()?;
        if n < 0 || n as usize > 1 << 24 {
            return Err(DecodeError(format!("bad length {n}")));
        }
        Ok(n as usize)
    }
    pub fn string(&mut self) -> DResult<String> {
        let n = self.len()?;
        let b = self.bytes(n)?;
        String::from_utf8(b.to_vec()).map_err(|_| DecodeError("invalid utf-8".into()))
    }
    pub fn byte_array(&mut self) -> DResult<&'a [u8]> {
        let n = self.len()?;
        self.bytes(n)
    }
    pub fn uuid(&mut self) -> DResult<u128> {
        Ok(u128::from_be_bytes(self.bytes(16)?.try_into().unwrap()))
    }
    pub fn long_array(&mut self) -> DResult<Vec<u64>> {
        let n = self.len()?;
        (0..n).map(|_| self.i64().map(|v| v as u64)).collect()
    }
    /// Packed block position.
    pub fn position(&mut self) -> DResult<(i32, i32, i32)> {
        Ok(unpack_position(self.i64()?))
    }
}

pub fn unpack_position(v: i64) -> (i32, i32, i32) {
    let x = (v >> 38) as i32;
    let y = ((v << 52) >> 52) as i32;
    let z = ((v << 26) >> 38) as i32;
    (x, y, z)
}

pub fn pack_position(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3FFFFFF) << 38) | (((z as i64) & 0x3FFFFFF) << 12) | ((y as i64) & 0xFFF)
}

#[derive(Default)]
pub struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn packet(id: i32) -> Self {
        let mut w = Writer::default();
        w.varint(id);
        w
    }
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(v as u8)
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i16(&mut self, v: i16) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn varint(&mut self, v: i32) -> &mut Self {
        write_varint(&mut self.buf, v);
        self
    }
    pub fn string(&mut self, s: &str) -> &mut Self {
        self.varint(s.len() as i32);
        self.buf.extend_from_slice(s.as_bytes());
        self
    }
    pub fn byte_array(&mut self, b: &[u8]) -> &mut Self {
        self.varint(b.len() as i32);
        self.buf.extend_from_slice(b);
        self
    }
    pub fn raw(&mut self, b: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(b);
        self
    }
    pub fn uuid(&mut self, v: u128) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn position(&mut self, x: i32, y: i32, z: i32) -> &mut Self {
        self.i64(pack_position(x, y, z))
    }
    pub fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }
}

pub fn write_varint(buf: &mut Vec<u8>, v: i32) {
    let mut v = v as u32;
    loop {
        if v & !0x7f == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
}

pub fn varint_len(v: i32) -> usize {
    let mut b = Vec::with_capacity(5);
    write_varint(&mut b, v);
    b.len()
}
