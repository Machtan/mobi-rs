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
use argonaut::{Parser, Arg};
use argonaut::ParseStatus::{Interrupted, Parsed};
use byteorder::{ReadBytesExt, BigEndian};
use common::*;
use palmdb::PalmDbHeader;
use mobi::MobiHeader;
use exth_tags::ExthTag;


fn read_mobi<R>(source: &mut R) -> Result<(), io::Error> where R: Read + Seek {    
    println!("====================== MOBI Information =====================");
        
    let palm_db_header = try!(PalmDbHeader::read_from(source));
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

/// Parses arguments for the 'info' subcommand
fn parse_info_subcommand(args: &[&str]) {
    let a_mobi_file = Arg::positional("mobi_file");
    
    let mut parser = Parser::new();
    parser.add(&a_mobi_file).unwrap();
    let usage = "Usage: mobitool info mobi_file";
    
    match parser.parse(args) {
        Ok(Parsed(parsed)) => {
            let mobi_file = parsed.positional("mobi_file").unwrap();
            
            let path = Path::new(mobi_file);
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(reason) => {
                    println!("Could not open file '{}'", mobi_file);
                    println!("{}", usage);
                    return ();
                },
            };
            let mut reader = BufReader::new(file);
            read_mobi(&mut reader).expect("Something went wrong:");
        },
        Ok(Interrupted(_)) => {},
        Err(reason) => {
            println!("Parse error: {:?}", reason);
            println!("{}", usage);
        }
    }
}

/*fn main() {
    let arg_vec: Vec<_> = env::args().skip(1).collect();
    let mut args: Vec<&str> = Vec::new();
    for arg in arg_vec.iter() {
        args.push(arg);
    }
    
    let a_command = Arg::positional("command");
    let a_args = Arg::optional_trail();
    let a_help = Arg::named_and_short("help", 'h').interrupt();
    let a_version = Arg::named("version").interrupt();

    let mut parser = Parser::new();
    parser.add(&a_command).unwrap();
    parser.add(&a_args).unwrap();
    parser.add(&a_help).unwrap();
    parser.add(&a_version).unwrap();
    let usage = "Usage: mobitool <info> [args...]";
    let help = "\
Required arguments:
command         The command to run. 
                info | (tbd...)

[ args... ]     Arguments for the subcommand.

Optional arguments:
--version       Show the SemVer version of this tool.
--help | -h     Show this help message.\
    ";
    
    match parser.parse(&args) {
        Ok(Parsed(parsed)) => {
            let command = parsed.positional("command").unwrap();
            let args = parsed.trail().unwrap();
            match command {
                "info" => parse_info_subcommand(&args),
                other => {
                    println!("'{}' isn't a valid command", other);
                    println!("{}", usage);
                }
            }
            
        },
        Ok(Interrupted("help")) => {
            println!("{}", usage);
            println!("");
            println!("{}", help);
        },
        Ok(Interrupted("version")) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        },
        Ok(Interrupted(_)) => unimplemented!(),
        Err(reason) => {
            println!("Parse error: {:?}", reason);
            println!("{}", usage);
        },
        
    }
}*/

fn main() {
    let palmdb_source = include_bytes!("palmdb_header.bin");
    let mut mobi_source = include_bytes!("mobi_header.bin");
    let mut exth_source = include_bytes!("exth_header.bin");
    let palm_db_header = PalmDbHeader::read_from(&mut &palmdb_source[..])
        .expect("could not read palm db header");
    palm_db_header.print_info();
    let mut header_buf: Vec<u8> = Vec::new();
    palm_db_header.write_to(&mut header_buf)
        .expect("could not write palm db header");

    for i in 0 .. palmdb_source.len() {
        if i > header_buf.len() {
            panic!("The written header lacks bytes at {}", i);
        }
        if header_buf[i] != palmdb_source[i] {
            panic!("Found difference in headers at byte {}!", i);
        }
    }
    //assert_eq!(&header_buf[..], &palmdb_source[..]);
    
    
}
