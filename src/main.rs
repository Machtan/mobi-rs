#![allow(unused)]

extern crate byteorder;
extern crate chrono;
extern crate argonaut;

#[macro_use]
mod common;
mod palmdb;
mod mobi;
mod exth_tags;

use std::env;
use std::fmt;
use std::io;
use std::io::{BufReader, Read, Write, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::process;
use argonaut::{ArgDef, parse, ParseError, help_arg, version_arg};
use byteorder::{ReadBytesExt, BigEndian};
use common::*;
use palmdb::PalmdbHeader;
use mobi::MobiHeader;
use exth_tags::ExthTag;

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum ErrorCode {
    ParseFailed = 1,
    Unspecified = 2,
}

static mut ERROR_CODE: Option<ErrorCode> = None;

fn main() {
    mobi_main();
    if let Some(code) = unsafe { ERROR_CODE } {
        process::exit(code as i32);
    }
}

fn print_mobi_info(filename: &str) {
    let path = Path::new(filename);
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(reason) => {
            println!("Could not open file '{}'", filename);
            unsafe {
                ERROR_CODE = Some(ErrorCode::Unspecified);
            }
            return ();
        },
    };
    let mut reader = BufReader::new(file);
    read_mobi(&mut reader).expect("Something went wrong:");
}

fn mobi_main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    
    let description = "
        Tool to work with e-books in the MOBI format.
    ";
    
    match parse("mobi", &args, vec![
        ArgDef::cmd("info", |program, args| {
            let mut filename = String::new();
            
            parse(program, args, vec![
                ArgDef::pos("filename", &mut filename)
                    .help("The file to print info about."),
                
                help_arg("
                    Prints all metadata of a MOBI file.
                "),
            ])?;
            
            print_mobi_info(&filename);
            
            Ok(())
        })
        .help("Prints all metadata of a MOBI file."),
        
        help_arg(description),
        version_arg(),
    ]) {
        Ok(_) => {},
        Err(ParseError::Interrupted(_)) => return,
        Err(_) => {
            unsafe {
                ERROR_CODE = Some(ErrorCode::ParseFailed);
            }
        },
    }
}


fn read_mobi<R>(source: &mut R) -> Result<(), io::Error> where R: Read + Seek {    
    println!("====================== MOBI Information =====================");
        
    let palm_db_header = try!(PalmdbHeader::read_from(source));
    palm_db_header.print_info();
    
    let first = palm_db_header.records[0];
    try!(source.seek(SeekFrom::Start(first.data_offset as u64)));
    
    let mobi_header = try!(MobiHeader::read_from(source));
    
    let has_exth_record = (mobi_header.exth_flags & 0x40) != 0;
    let tags = if has_exth_record {
        try!(exth_tags::read_from(source))
    } else {
        Vec::new()
    };
    println!("EXTH tags:");
    for tag in tags.iter() {
        println!("- {:?}", tag);
    }
    
    read_compressed_record(source, palm_db_header.records[1].data_offset);
    read_compressed_record(source, palm_db_header.records[2].data_offset);
    
    Ok(())
}

fn read_compressed_record<R>(source: &mut R, data_offset: u32)
        -> Result<(), io::Error> where R: Read + Seek {
    println!("\n===========================================================\n");
    let start = data_offset as u64;
    try!(source.seek(SeekFrom::Start(start)));
    
    let mut bytes_read: u16 = 0;
    let record_size: u16 = 4096;
    let mut output: Vec<u8> = Vec::new();
    
    while bytes_read < record_size {
        let byte = try!(source.read_u8());
        bytes_read += 1;
        match byte {
            literal @ 0x00 | literal @ 0x09 ... 0x7F => {
                output.push(literal);
            },
            count @ 0x01 ... 0x08 => {
                let mut buf: Vec<u8> = Vec::new();
                try!(source.take(count as u64).read_to_end(&mut buf));
                output.extend_from_slice(&buf);
                bytes_read += count as u16;
                
            },
            0x80 ... 0xBF => {
                let mut vec: Vec<u8> = Vec::new();
                vec.push(byte & 0b00111111);
                let second = try!(source.read_u8());
                bytes_read += 1;
                vec.push(second);
                let pair = try!(read_u16_be(&mut &vec[..]));
                
                let distance = pair >> 3;
                
                let length = ((second & 0b00000111) + 3) as usize;
                
                //println!("Distance: {}, Length: {}", distance, length);
                let pos = output.len() - distance as usize;
                let mut end = pos + length;
                // This is taken from the Calibre implementation
                let mut buf: Vec<u8> = Vec::new();
                
                while end > output.len() {
                    buf.extend_from_slice(&output[pos..]);
                    end -= output.len() - pos;
                }
                
                buf.extend_from_slice(&output[pos .. end]);
                print!("{:05} | {:05} + [{} : {}] : [ ", 
                    start + bytes_read as u64 - 2, output.len(),
                    pos, pos + length);
                for b in buf.iter() {
                    print!("{:x} ", b);
                }
                println!("]");
                
                output.extend_from_slice(&buf);
                
            },
            0xC0 ... 0xFF => {
                let ch = byte ^ 0x80;
                //println!("Writing space and '{}'", ch as char);
                output.push(' ' as u8);
                output.push(ch);
            },
            _ => {
                unreachable!();
            }            
        }
    }
    
    println!("Writing buffer...");
    let mut test_file = try!(File::create("books/test.txt"));
    try!(test_file.write_all(&output[..]));
    println!("Done!");
    
    //println!("Buffer:\n");
    //println!("{:?}", &output);
    let string = String::from_utf8(output).expect("Invalid UTF8 in buffer :c");
    println!("Decompressed text:\n");
    println!("{}", string);
    
    Ok(())
}

fn compare_bytes(actual: &[u8], expected: &[u8]) {
    for i in 0 .. expected.len() {
        if i > actual.len() {
            panic!("The actual buffer lacks bytes at {}", i);
        }
        if actual[i] != expected[i] {
            panic!("Found difference at byte {}!", i);
        }
    }
}

fn test_headers() {
    let palmdb_source = include_bytes!("palmdb_header.bin");
    let mut mobi_source = include_bytes!("mobi_header.bin");
    let mut exth_source = include_bytes!("exth_header.bin");
    
    let palm_db_header = PalmdbHeader::read_from(&mut &palmdb_source[..])
        .expect("Could not read palm db header");
    palm_db_header.print_info();
    let mut header_buf: Vec<u8> = Vec::new();
    palm_db_header.write_to(&mut header_buf)
        .expect("could not write palm db header");
    compare_bytes(&header_buf[..], palmdb_source);
    println!("");
    
    let mobi_header = MobiHeader::read_from(&mut &mobi_source[..])
        .expect("Could not read mobi header");
    mobi_header.print_info();
    
    //assert_eq!(&header_buf[..], &palmdb_source[..]);
    
    
}
