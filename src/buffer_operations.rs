use crate::error::{EofError, Result};

pub fn read_slice<'a>(slice: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    if slice.len() < len {
        *slice = &[];
        Err(EofError)
    } else {
        let (head, tail) = slice.split_at(len);
        *slice = tail;
        Ok(head)
    }
}

pub fn read_array<'a, const N: usize>(slice: &mut &'a [u8]) -> Result<&'a [u8; N]> {
    let bytes = read_slice(slice, N)?;
    Ok(bytes.try_into().unwrap())
}

pub fn read_u8(slice: &mut &[u8]) -> Result<u8> {
    if slice.is_empty() {
        Err(EofError)
    } else {
        let res = slice[0];
        *slice = &slice[1..];
        Ok(res)
    }
}

pub fn read_u16(slice: &mut &[u8]) -> Result<u16> {
    let bytes = read_slice(slice, 2)?;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

pub fn read_u32(slice: &mut &[u8]) -> Result<u32> {
    let bytes = read_slice(slice, 4)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

pub fn read_u64(slice: &mut &[u8]) -> Result<u64> {
    let bytes = read_slice(slice, 8)?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

pub fn read_var_int(slice: &mut &[u8]) -> Result<u64> {
    let first = read_u8(slice)?;
    let n = match first {
        0xfd => read_u16(slice)? as u64,
        0xfe => read_u32(slice)? as u64,
        0xff => read_u64(slice)?,
        n => n as u64,
    };
    Ok(n)
}
