
extern crate byteorder;

use std::fmt;
use std::io;
use std::io::{Read, Seek};
use byteorder::{ReadBytesExt};
use common::*;

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

/// Not quite sure what these are for
#[derive(Debug)]
pub struct Indices {
    pub orthographic: Option<u32>,
    pub inflection: Option<u32>,
    pub names: Option<u32>,
    pub keys: Option<u32>,
}

/// Uh, not sure about this either.
#[derive(Debug)]
pub struct HuffmanEncodingInfo {
    pub record_offset: u32,
    pub record_count: u32,
    pub table_offset: u32,
    pub table_length: u32,
}

/// Info for dictionary e-books, I guess.
#[derive(Debug)]
pub struct DictionaryInfo {
    pub input: Language,
    pub output: Language,
}

/// Info about the DRM of the content.
#[derive(Debug)]
pub struct DrmInfo {
    pub offset: Option<u32>,
    pub count: u32,
    pub size: u32,
    pub flags: u32,
}

/// What even is this?
#[derive(Debug)]
pub struct CompilationInfo {
    pub data_section_count: u32,
    pub data_sections: Option<u32>,
}

/// The header 
#[derive(Debug)]
pub struct MobiHeader {
    pub compression: CompressionType,
    pub uncompressed_text_length: u32,
    pub encryption: EncryptionType,
    pub content_type: MobiType,
    pub text_encoding: TextEncoding,
    pub mobi_id: u32,
    pub mobi_version: u32,
    pub indices: Indices,
    pub locale: Language,
    pub dictionary: DictionaryInfo,
    pub huffman_encoding: HuffmanEncodingInfo,
    pub exth_flags: u32,
    pub drm: DrmInfo,
    pub compilation: CompilationInfo,
}

impl MobiHeader {
    /// Attempts to read a MOBI header from the given source
    pub fn read_from<R>(source: &mut R) 
            -> Result<MobiHeader, io::Error> where R: Read + Seek {
        
        let compression = CompressionType::from(try!(read_u16_be(source)));
        try!(discard(source, 2)); // Ignore unused field
        let uncompressed_text_length = try!(read_u32_be(source));
        let record_count = try!(read_u16_be(source));
        let record_size = try!(read_u16_be(source));
        assert_eq!(record_size, 4096);
        let encryption = EncryptionType::from(try!(read_u16_be(source)));
        let unknown = try!(read_u16_be(source));
        println!("Compression: {:?}", compression);
        println!("Text length: {}", uncompressed_text_length);
        println!("Record count: {}", record_count);
        println!("Encryption type: {:?}", encryption);
    
        let magic = try!(read_string(source, 4));
        assert_eq!(magic, String::from("MOBI"));
    
        let header_len = try!(read_u32_be(source));
        let content_type = MobiType::from(try!(read_u32_be(source)));
        println!("Magic: {}, header len: {}, type: {:?}", magic, header_len, 
            content_type);
    
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
    
        let indices = Indices {
            orthographic: orthographic_index,
            inflection: inflection_index,
            names: index_names,
            keys: index_keys,
        };

        let huffman_encoding = HuffmanEncodingInfo {
            record_offset: huffman_record_offset,
            record_count: huffman_record_count,
            table_offset: huffman_table_offset,
            table_length: huffman_table_length,
        };

        let dictionary = DictionaryInfo {
            input: dict_input_language,
            output: dict_output_language,
        };

        let drm = DrmInfo {
            offset: drm_offset,
            count: drm_count,
            size: drm_size,
            flags: drm_flags,
        };

        let compilation = CompilationInfo {
            data_section_count: compilation_data_section_count,
            data_sections: number_of_compilation_data_sections,
        };
    
        Ok(MobiHeader {
            compression: compression,
            uncompressed_text_length: uncompressed_text_length,
            encryption: encryption,
            content_type: content_type,
            text_encoding: text_encoding,
            mobi_id: mobi_id,
            mobi_version: mobi_version,
            indices: indices,
            locale: locale,
            dictionary: dictionary,
            huffman_encoding: huffman_encoding,
            exth_flags: exth_flags,
            drm: drm,
            compilation: compilation,
        })
    }
}