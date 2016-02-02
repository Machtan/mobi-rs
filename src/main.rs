#![allow(unused)]

extern crate byteorder;
extern crate chrono;
extern crate argonaut;

#[macro_use]
mod common;
mod palmdb;
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


valued_enum! {
    CompressionType : u16 {
        None = 0,
        PalmDOC = 2,
        HUFFCDIC = 17480
    }
}

valued_enum! {
    EncryptionType: u16 {
        None = 0,
        OldMobiPocket = 1,
        MobiPocket = 2
    }
}


valued_enum! {
    MobiType : u32 {
        MobiPocketBook = 2,
        PalmDocBook = 3,
        Audio = 4,
        MaybeMobiPocket = 232,
        KF8 = 248,
        News = 257,
        NewsFeed = 258,
        NewsMagazine = 259,
        PICS = 513,
        WORD = 514,
        XLS = 515,
        PPT = 516,
        TEXT = 517,
        HTML = 518
    }
}

valued_enum! {
    TextEncoding : u32 {
        Latin1 = 1252,
        UTF8 = 65001
    }
}



fn read_mobi<R>(source: &mut R) -> Result<(), io::Error> where R: Read + Seek {    
    println!("====================== MOBI Information =====================");
        
    let palm_db_header = try!(PalmDbHeader::read_from(source));
    palm_db_header.print_info();
    
    // ========================== PalmDoc header ===============================
    
    let first = palm_db_header.records[0];
    try!(source.seek(SeekFrom::Start(first.data_offset as u64)));
    
    let compression = CompressionType::from(try!(read_u16_be(source)));
    try!(discard(source, 2)); // Ignore unused field
    let text_length = try!(read_u32_be(source));
    let record_count = try!(read_u16_be(source));
    let record_size = try!(read_u16_be(source));
    assert_eq!(record_size, 4096);
    let encryption = EncryptionType::from(try!(read_u16_be(source)));
    let unknown = try!(read_u16_be(source));
    println!("Compression: {:?}", compression);
    println!("Text length: {}", text_length);
    println!("Record count: {}", record_count);
    println!("Encryption type: {:?}", encryption);
    
    // ============================ MOBI header ================================
    let magic = try!(read_string(source, 4));
    assert_eq!(magic, String::from("MOBI"));
    
    let header_len = try!(read_u32_be(source));
    let mobi_type = MobiType::from(try!(read_u32_be(source)));
    println!("Magic: {}, header len: {}, type: {:?}", magic, header_len, 
        mobi_type);
    
    let text_encoding = TextEncoding::from(try!(read_u32_be(source)));
    println!("Text encoding: {:?}", text_encoding);
    let mobi_id = try!(read_u32_be(source));
    let mobi_version = try!(read_u32_be(source));
    println!("Id: {}, Version: {}", mobi_id, mobi_version);
    
    let orthographic_index = try!(read_unmaxed_u32(source));
    let inflection_index = try!(read_unmaxed_u32(source)); 
    let index_names = try!(read_unmaxed_u32(source));
    let index_keys = try!(read_unmaxed_u32(source));
    println!("Indices:");
    println!("- Orthographic:   {:?}", orthographic_index); 
    println!("- Inflection:     {:?}", inflection_index);
    println!("- Names:          {:?}", index_names);
    println!("- Keys:           {:?}", index_keys);
    
    println!("Extra indices:");
    let mut extra_indices = Vec::new();
    for i in 0..6 {
        let index = try!(read_unmaxed_u32(source));
        extra_indices.push(index);
        println!("- {}: {:?}", i, index);
    }
    
    let first_record = try!(read_u32_be(source));
    let full_name_offset = try!(read_u32_be(source));
    let full_name_length = try!(read_u32_be(source));
    println!("First record: {}", first_record);
    println!("Name offset: {}, length: {}", full_name_offset, full_name_length);
    
    let locale = Language::from(try!(read_u32_be(source)));
    println!("Locale: {:?}", locale);
    let dict_input_language = Language::from(try!(read_u32_be(source)));
    let dict_output_language = Language::from(try!(read_u32_be(source)));
    println!("Dict input/output: {:?} -> {:?}", dict_input_language, 
        dict_output_language);
    
    let min_version = try!(read_u32_be(source));
    println!("Min supported version: {}", min_version);
    
    let first_image_record = try!(read_u32_be(source));
    println!("First image record: {}", first_image_record);
    
    let huffman_record_offset = try!(read_u32_be(source));
    let huffman_record_count = try!(read_u32_be(source));
    let huffman_table_offset = try!(read_u32_be(source));
    let huffman_table_length = try!(read_u32_be(source));
    println!("Huffman record offset: {}", huffman_record_offset);
    println!("Huffman record count: {}", huffman_record_count);
    println!("Huffman table offset: {}", huffman_table_offset);
    println!("Huffman table length: {}", huffman_table_length);
    
    let exth_flags = try!(read_u32_be(source));
    let has_exth_record = (exth_flags & 0x40) != 0;
    println!("Exth flags: {:b}, Has EXTH: {}", exth_flags, has_exth_record);
    
    try!(discard(source, 32)); // Unknown
    try!(discard(source, 4)); // Unknown (0xFFFFFFFF)
    
    let drm_offset = try!(read_unmaxed_u32(source));
    let drm_count = try!(read_unmaxed_u32(source)).unwrap_or(0);
    let drm_size = try!(read_u32_be(source));
    let drm_flags = try!(read_u32_be(source));
    println!("DRM");
    println!("- Offset: {:?}", drm_offset);
    println!("- Count:  {:?}", drm_count);
    println!("- Size:   {}", drm_size);
    println!("- Flags:  {:b}", drm_flags);
    
    try!(discard(source, 8)); // Unknown (0x0000000000000000)
    
    let text_record = try!(read_u16_be(source));
    let last_record = try!(read_u16_be(source));
    println!("Text record: {}", text_record);
    println!("Last record: {}", last_record);
    
    try!(discard(source, 4)); // Unknown (0x00000001)
    
    let fcis_record_number = try!(read_u32_be(source));
    let fcis_record_count_maybe = try!(read_u32_be(source)); // (0x00000001)
    println!("FCIS record: Number: {}, Count: {}", fcis_record_number,
        fcis_record_count_maybe);
    
    let flis_record_number = try!(read_u32_be(source));
    let flis_record_count_maybe = try!(read_u32_be(source)); // (0x00000001)
    println!("FLIS record: Number: {}, Count: {}", flis_record_number,
        flis_record_count_maybe);
    
    try!(discard(source, 8)); // (0x0000000000000000)
    try!(discard(source, 4)); // (0xFFFFFFFF)
    
    let compilation_data_section_count = try!(read_u32_be(source)); // (0x00000000)
    let number_of_compilation_data_sections = try!(read_unmaxed_u32(source)); // (0xFFFFFFFF)
    println!("Compilation data sections: ???: {}, ???: {:?}",
        compilation_data_section_count, number_of_compilation_data_sections);
    
    try!(discard(source, 4)); // (0xFFFFFFFF)
    
    let extra_record_data_flags = try!(read_u32_be(source));
    println!("Extra record data flags: {:b}", extra_record_data_flags);
    
    let indx_record_offset = try!(read_unmaxed_u32(source));
    println!("INDX record offset: {:?}", indx_record_offset);
    
    if header_len > 232 {
        println!("Discarding header bytes after offset 232...");
        //try!(discard(source, 20)); // 5x (0xFFFFFFFF)
        //try!(discard(source, 4)); // (0)
        try!(discard(source, (header_len - 232) as u64));
    }
    
    // ============================ EXTH header ================================
    let tags = if has_exth_record {
        try!(exth_tags::read_from(source))
    } else {
        Vec::new()
    };
    println!("EXTH tags:");
    for tag in tags.iter() {
        println!("- {:?}", tag);
    }
    
    // =========================== Data records ================================
    // Read the actual data
    //let record = records[1];
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

fn main() {
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
}
