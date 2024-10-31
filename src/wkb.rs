use crate::XY;
use std::io::{Result, Write /*,ErrorKind,Error*/};
//use geos::Geom;

/*
pub fn write_uint16<W: Write>(w: &mut W, i: usize) -> Result<()> {
    w.write_all(&[(i & 255) as u8, ((i >> 8) & 255) as u8])
}

pub fn write_int32<W: Write>(w: &mut W, i: i32) -> Result<()> {
    write_uint32(w, i as u32)
}

pub fn write_int64<W: Write>(w: &mut W, i: i64) -> Result<()> {
    write_uint64(w, i as u64)
}*/

pub fn write_uint32<W: Write>(w: &mut W, i: u32) -> Result<()> {
    w.write_all(&[
        (i & 255) as u8,
        ((i >> 8) & 255) as u8,
        ((i >> 16) & 255) as u8,
        ((i >> 24) & 255) as u8
    ])
}

pub fn write_uint64<W: Write>(w: &mut W, i: u64) -> Result<()> {
    w.write_all(&[
        (i & 255) as u8,
        ((i >> 8) & 255) as u8,
        ((i >> 16) & 255) as u8,
        ((i >> 24) & 255) as u8,
        ((i >> 32) & 255) as u8,
        ((i >> 40) & 255) as u8,
        ((i >> 48) & 255) as u8,
        ((i >> 56) & 255) as u8,
        
    ])
}

pub fn write_f64<W: Write>(w: &mut W, f: f64) -> Result<()> {
    write_uint64(w, f.to_bits())
}





pub fn prep_wkb(transform: bool, with_srid: bool, ty: u32, ln: usize) -> Result<Vec<u8>> {
    let l = 1 + 4 + (if with_srid { 4 } else { 0 }) + ln;
    let mut res = Vec::with_capacity(l);

    res.push(1);
    if with_srid {
        write_uint32(&mut res, ty + (32 << 24))?;
        write_uint32(&mut res, if transform { 3857 } else { 4326 })?;
    } else {
        write_uint32(&mut res, ty)?;
    }

    Ok(res)
}

pub fn write_point<W: Write>(w: &mut W, xy: &XY) -> Result<()> {
    write_f64(w, xy.x)?;
    write_f64(w, xy.y)
}

pub fn write_ring<W: Write, Iter: Iterator<Item = XY>>(
    w: &mut W,
    ln: usize,
    iter: Iter,
) -> Result<()> {
    write_uint32(w, ln as u32)?;
    for i in iter {
        write_point(w, &i)?;
    }
    Ok(())
}
/*
pub trait AsWkb {
    fn as_wkb(&self, srid: Option<u32>) -> Result<Vec<u8>>;
}*/
