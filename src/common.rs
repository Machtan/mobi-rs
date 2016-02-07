//! Utilities for byte-reading, struct definition macros....

extern crate byteorder;
use std::io;
use std::io::{Read, Write};


use byteorder::{ReadBytesExt, BigEndian, WriteBytesExt};

pub fn read_unmaxed_u32(source: &mut Read)
        -> Result<Option<u32>, byteorder::Error> {
    match try!(source.read_u32::<BigEndian>()) {
        0xFFFFFFFF => Ok(None),
        other => Ok(Some(other)),
    }
}

pub fn read_u32_be(source: &mut Read) -> Result<u32, byteorder::Error> {
    source.read_u32::<BigEndian>()
}

pub fn read_i32_be(source: &mut Read) -> Result<i32, byteorder::Error> {
    source.read_i32::<BigEndian>()
}


pub fn read_u16_be(source: &mut Read) -> Result<u16, byteorder::Error> {
    source.read_u16::<BigEndian>()
}

pub fn discard(source: &mut Read, bytes: u64) -> Result<(), io::Error> {
    let mut buf = Vec::new();
    try!(source.take(bytes).read_to_end(&mut buf));
    Ok(())
}

pub fn read_until_zero(buf: &[u8]) -> &[u8] {
    buf.split(|n| *n == 0).nth(0).unwrap()
}

pub fn read_string(source: &mut Read, len: u64) -> Result<String, io::Error> {
    let mut buf = String::new();
    try!(source.take(len).read_to_string(&mut buf));
    Ok(buf)
}

pub fn write_u32_be(source: &mut Write, val: u32) -> Result<(), byteorder::Error> {
    source.write_u32::<BigEndian>(val)
}

pub fn write_i32_be(source: &mut Write, val: i32) -> Result<(), byteorder::Error> {
    source.write_i32::<BigEndian>(val)
}

pub fn write_u16_be(source: &mut Write, val: u16) -> Result<(), byteorder::Error> {
    source.write_u16::<BigEndian>(val)
}

pub fn write_u24_be(source: &mut Write, val: u32) -> Result<(), byteorder::Error> {
    source.write_uint::<BigEndian>(val as u64, 3)
}

/// Creates an enum with the given variants, where each variant can be
/// converted to/from associated values of the specified type.
/// The struct has a 'from(value)' member and a 'value()' member.
macro_rules! valued_enum {
    (
        $name:ident : $value_type:ty {
            $(
                $variant:ident = $value:expr
            ),*
        }
    ) => {
        #[derive(Debug, PartialEq, Hash)]
        pub enum $name {
            $(
                $variant,
            )*
            Unknown($value_type),
        }
        
        impl $name {
            pub fn from(value: $value_type) -> $name {
                match value {
                    $(
                        $value => $name::$variant,
                    )*
                    _ => $name::Unknown(value),
                }
            }
            
            pub fn value(&self) -> $value_type {
                match *self {
                    $(
                        $name::$variant => $value,
                    )*
                    $name::Unknown(value) => value,
                }
            }
        }
    }
}

valued_enum! {
    Language : u32 {
        En = 0x09,
        EnUs = 0x0904,
        EnUk = 0x0908
    }
}
