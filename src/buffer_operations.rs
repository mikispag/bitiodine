use preamble::*;

pub fn read_slice<'a>(slice: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    if slice.len() < len {
        *slice = &[];
        Err(EofError)
    } else {
        let res = &slice[..len];
        *slice = &slice[len..];
        Ok(res)
    }
}

macro_rules! read_array {
    ($slice:expr, $len:expr) => {{
        ::buffer_operations::read_slice($slice, $len).map(|slice| array_ref!(slice, 0, $len))
    }};
}

pub fn read_u8(slice: &mut &[u8]) -> Result<u8> {
    if slice.len() == 0 {
        Err(EofError)
    } else {
        let res = slice[0];
        *slice = &slice[1..];
        Ok(res)
    }
}

pub fn read_u16(slice: &mut &[u8]) -> Result<u16> {
    Ok(LittleEndian::read_u16(read_slice(slice, 2)?))
}

pub fn read_u32(slice: &mut &[u8]) -> Result<u32> {
    Ok(LittleEndian::read_u32(read_slice(slice, 4)?))
}

pub fn read_u64(slice: &mut &[u8]) -> Result<u64> {
    Ok(LittleEndian::read_u64(read_slice(slice, 8)?))
}

pub fn read_var_int(slice: &mut &[u8]) -> Result<u64> {
    let n = match read_array!(slice, 1)?[0] {
        0xfd => read_u16(slice)? as u64,
        0xfe => read_u32(slice)? as u64,
        0xff => read_u64(slice)?,
        n => n as u64,
    };
    Ok(n)
}
