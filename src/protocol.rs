// SPDX-License-Identifier: AGPL-3.0-or-later
use std::io::{self, Read, Write};
pub const VERSION: i32 = 777;
const MAX: usize = 1 << 20;
fn bad(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.to_string())
}
pub fn varint_read(r: &mut impl Read) -> io::Result<i32> {
    let mut n = 0u32;
    for shift in 0..5 {
        let mut b = [0];
        r.read_exact(&mut b)?;
        if shift == 4 && b[0] > 0x0f {
            return Err(bad("varint overflow"));
        }
        n |= ((b[0] & 127) as u32) << (7 * shift);
        if b[0] & 128 == 0 {
            return Ok(n as i32);
        }
    }
    Err(bad("overlong varint"))
}
pub fn varint_write(mut n: i32, out: &mut Vec<u8>) {
    loop {
        if (n as u32) & !127 == 0 {
            out.push(n as u8);
            return;
        }
        out.push((n as u8 & 127) | 128);
        n = ((n as u32) >> 7) as i32;
    }
}
pub fn frame_read(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let n = varint_read(r)?;
    if n < 0 || n as usize > MAX {
        return Err(bad("invalid frame length"));
    }
    let mut buf = vec![0; n as usize];
    r.read_exact(&mut buf)?;
    Ok(buf)
}
pub fn frame_write(w: &mut impl Write, contents: &[u8]) -> io::Result<()> {
    if contents.len() > MAX {
        return Err(bad("frame too long"));
    }
    let mut buf = vec![];
    varint_write(contents.len() as i32, &mut buf);
    buf.extend_from_slice(contents);
    w.write_all(&buf)
}
pub fn write_string(text: &str, b: &mut Vec<u8>) {
    varint_write(text.len() as i32, b);
    b.extend_from_slice(text.as_bytes());
}
pub fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    pub fn varint(&mut self) -> io::Result<i32> {
        varint_read(self)
    }
    pub fn bytes(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or_else(|| bad("overflow"))?;
        let chunk = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| bad("truncated"))?;
        self.pos = end;
        Ok(chunk)
    }
    pub fn text(&mut self, max: usize) -> io::Result<&'a str> {
        let len = self.varint()?;
        if len < 0 || len as usize > max {
            return Err(bad("string too long"));
        }
        std::str::from_utf8(self.bytes(len as usize)?).map_err(|_| bad("bad utf8"))
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}
impl Read for Cursor<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(self.remaining());
        out[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn varints() {
        for n in [0, 1, 127, 128, 255, 777, 16384, i32::MAX, -1] {
            let mut buf = vec![];
            varint_write(n, &mut buf);
            assert_eq!(varint_read(&mut &buf[..]).unwrap(), n);
        }
    }
    #[test]
    fn rejects_overflow() {
        assert!(varint_read(&mut &[255, 255, 255, 255, 16][..]).is_err());
    }
    #[test]
    fn rejects_oversized() {
        let mut x = vec![];
        varint_write((MAX + 1) as i32, &mut x);
        assert!(frame_read(&mut &x[..]).is_err());
    }
    #[test]
    fn json_escape() {
        assert_eq!(json("a\n\""), "\"a\\n\\\"\"");
    }
}
