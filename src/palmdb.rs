//! Palm Database functionality

extern crate chrono;
extern crate byteorder;

use std::fmt;
use std::io;
use std::io::{Read, Write};
use chrono::{NaiveDateTime};
use byteorder::{ReadBytesExt, BigEndian, WriteBytesExt};
use common::*;

#[derive(Debug, Clone, Copy)]
pub struct Record {
    pub id: u32,
    pub data_offset: u32,
    pub attributes: u8,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Id: {:4}, offset: {:10}, attributes: {:b}", 
            self.id, self.data_offset, self.attributes)
    }
}

valued_enum! {
    PalmDbType : &'static str {
        Mobi = "BOOKMOBI"
    }
}

/// The Palm Database format header. 
/// The struct only supports MOBI.
#[derive(Debug)]
pub struct PalmdbHeader {
    pub name: [u8; 31], // Null-terminated string * by the program *
    pub attributes: u16,
    pub version: u16,
    pub creation_date: NaiveDateTime,
    pub modification_date: NaiveDateTime,
    pub backup_date: NaiveDateTime,
    pub modification_number: u32,
    pub app_info_offset: Option<u32>,
    pub sort_info_offset: Option<u32>,
    pub content_type: PalmDbType,
    pub unique_id_seed: u32,
    pub next_record_list_id: u32,
    pub records: Vec<Record>,
}
impl PalmdbHeader {
    
    /// Reads a Palm database header from the given source
    pub fn read_from(source: &mut Read) -> Result<PalmdbHeader, io::Error> {
        let mut name_buf = [0; 32];
        try!(source.read(&mut name_buf));
        let mut name: [u8; 31] = [0; 31];
        for i in 0..31 {
            name[i] = name_buf[i];
        }
    
        let attributes = try!(read_u16_be(source));
    
        let version = try!(read_u16_be(source));
    
        let creation_timestamp = try!(read_i32_be(source)) as i64;
        let creation_date = NaiveDateTime::from_timestamp(creation_timestamp, 0);
        let mod_timestamp = try!(read_i32_be(source)) as i64;
        let modification_date = NaiveDateTime::from_timestamp(mod_timestamp, 0);
        let backup_timestamp = try!(read_i32_be(source)) as i64;
        let backup_date = NaiveDateTime::from_timestamp(backup_timestamp, 0);
    
        let modification_number = try!(read_u32_be(source));
    
        let app_info_offset = try!(read_u32_be(source));
        let app_info_offset = if app_info_offset == 0 {
            None
        } else {
            Some(app_info_offset)
        };
        let sort_info_offset = try!(read_u32_be(source));
        let sort_info_offset = if sort_info_offset == 0 {
            None
        } else {
            Some(sort_info_offset)
        };
    
        let file_type = try!(read_string(source, 4));
        let creator_program = try!(read_string(source, 4));
        assert_eq!(file_type, String::from("BOOK"));
        assert_eq!(creator_program, String::from("MOBI"));
        let content_type = PalmDbType::Mobi;
    
        let unique_id_seed = try!(read_u32_be(source));
        let next_record_list_id = try!(read_u32_be(source));
    
        
        let mut records = Vec::new();
        let number_of_records = try!(read_u16_be(source));
        
        for i in 0..number_of_records {
            let data_offset = try!(read_u32_be(source));
            let attributes = try!(source.read_u8());
            let id = try!(source.read_uint::<BigEndian>(3)) as u32;
            records.push( Record { id: id, data_offset: data_offset, 
                attributes: attributes } );
        }
    
        Ok(PalmdbHeader {
            name: name, // Null-terminated string * by the program *
            attributes: attributes,
            version: version,
            creation_date: creation_date,
            modification_date: modification_date,
            backup_date: backup_date,
            modification_number: modification_number,
            app_info_offset: app_info_offset,
            sort_info_offset: sort_info_offset,
            content_type: content_type,
            unique_id_seed: unique_id_seed,
            next_record_list_id: next_record_list_id,
            records: records,
        })
    }
    
    /// Prints the relevant information about this database header
    pub fn print_info(&self) {
        println!("===== PalmDB header =====");
        let name = String::from_utf8_lossy(read_until_zero(&self.name));
        println!("PalmDB name: {}", name);
        println!("Version: {}", self.version);
        println!("Created:  {:?}", self.creation_date);
        println!("Modified: {:?}", self.modification_date);
        println!("Modification number: {}", self.modification_number);
        println!("Number of records: {}", self.records.len());
        println!("Info of the 10 first records:");
        for i in 0..10 {
            if i < self.records.len() {
                println!("{}", self.records[i]);
            }
        }
    }
    
    pub fn write_to(&self, output: &mut Write) -> Result<(), io::Error> {
        try!(output.write_all(&self.name[..]));
        try!(output.write_all(&[0x00]));
        try!(write_u16_be(output, self.attributes));
        try!(write_u16_be(output, self.version));
        try!(write_u32_be(output, self.creation_date.timestamp() as u32));
        try!(write_u32_be(output, self.modification_date.timestamp() as u32));
        try!(write_u32_be(output, self.backup_date.timestamp() as u32));
        try!(write_u32_be(output, self.modification_number));
        try!(write_u32_be(output, self.app_info_offset.unwrap_or(0)));
        try!(write_u32_be(output, self.sort_info_offset.unwrap_or(0)));
        try!(output.write_all(self.content_type.value().as_bytes()));
        try!(write_u32_be(output, self.unique_id_seed));
        try!(write_u32_be(output, self.next_record_list_id));
        try!(write_u16_be(output, self.records.len() as u16));
        for record in self.records.iter() {
            try!(write_u32_be(output, record.data_offset));
            try!(output.write_all(&[record.attributes]));
            try!(write_u24_be(output, record.id));
        }
        try!(output.write_all(&[0x00, 0x00])); // Padding (from mobileread)
        
        Ok(())
    }
}