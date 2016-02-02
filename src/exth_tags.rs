
use std::io;
use std::io::Read;
use common;
use common::*;


/// Reads a bunch of EXTH tags from the given input source.
pub fn read_from(source: &mut Read) -> Result<Vec<ExthTag>, io::Error> {
    let magic_exth = try!(read_string(source, 4));
    //println!("EXTH magic: {}", magic_exth);
    assert_eq!(magic_exth, String::from("EXTH"));
    
    let header_len = try!(read_u32_be(source));
    let exth_record_count = try!(read_u32_be(source));
    //println!("EXTH Header length: {}", header_len);
    //println!("EXTH records: {}", exth_record_count);
    
    // Read the EXTH records
    let mut exth_tags = Vec::new();
    let total_record_len = header_len - 12;
    {
        let mut record_source = source.take(total_record_len as u64);
        for i in 0..exth_record_count {
            let tag = try!(ExthTag::read_from(&mut record_source));
            exth_tags.push(tag);
        }
    }
    
    // Null bytes to pad the EXTH header to a multiple of four bytes
    try!(discard(source, (header_len % 4) as u64));
    Ok(exth_tags)
}

// Taken from the mobileread wiki
valued_enum! {
    ExthType : u32 {
        DRMServerId = 1,
        DRMCommerveId = 2,
        DRMEbookbaseBookId = 3,
        Author = 100,           // <dc:Creator>
        Publisher = 101,        // <dc:Publisher>
        Imprint = 102,          // <Imprint>
        Description = 103,      // <dc:Description>
        ISBN = 104,             // <dc:Identifier scheme='ISBN'>
        Subject = 105,          // <dc:Subject> Could appear multiple times 	
        PublishingDate = 106,   // <dc:Date>
        Review = 107,           // <Review>
        Contributor = 108,      // <dc:Contributor>
        Rights = 109,           // <dc:Rights>
        SubjectCode = 110,      // <dc:Subject BASICCode="subjectcode">
        Type = 111,             // <dc:Type>
        Source = 112,           // <dc:Source>
        ASIN = 113,     // Kindle Paperwhite labels books with 
                        // "Personal" if they don't have this record.
        VersionNumber = 114,
        IsSample = 115, // 0x0001 if the book content is only a sample of the 
                        // full book
        StartReadingAtOffset = 116, // Position (4-byte offset) in file at 
                                    // which to open when first opened
        AdultOnly = 117,    // <Adult> Mobipocket Creator adds this if Adult 
                            // only is checked on its GUI; contents: "yes" 	
        RetailPrice = 118,  // <SRP> As text, e.g. "4.99" 	
        RetailPriceCurrency = 119,  // <SRP Currency="currency"> 
                                    // As text, e.g. "USD"
        KF8BoundaryOffset = 121,
        ResourceCount = 125,
        KF8CoverURI = 129,
        UsedButUnknown = 131,
        DictionaryShortName = 200, // <DictionaryVeryShortName> As text
        CoverOffset = 201,  // <EmbeddedCover> Add to first image field in Mobi 
                            // Header to find PDB record containing the cover image 	
	    ThumbnailOffset = 202,  // Add to first image field in Mobi Header to 
                                // find PDB record containing the thumbnail 
                                // cover image
        HasFakeCover = 203,
        CreatorSoftware = 204,  // Known Values: 1=mobigen, 2=Mobipocket Creator
                                // 200=kindlegen (Windows), 201=kindlegen (Linux)
                                // 202=kindlegen (Mac).
        CreatorMajorVersion = 205, // u32
        CreatorMinorVersion = 206, // u32
        CreatorBuildNumber = 207, // u32
        Watermark = 208,
        TamperProofKeys = 209,  // Used by the Kindle (and Android app) for 
                                // generating book-specific PIDs.
        FontSignature = 300,
        ClippingLimit = 401,    // Integer percentage of the text allowed to be
                                // clipped. Usually 10.
        PublisherLimit = 402,
        UsedButUnknown2 = 403,
        TextToSpeechFlag = 404,	// 1 - Text to Speech disabled 
                                // 0 - Text to Speech enabled
        MaybeRentBorrowFlag = 405,  // 1 in this field seems to indicate a 
                                    // rental book 
        RentBorrowExpirationDate = 406, // If this field is removed from a 
                                        // rental, the book says it expired in 
                                        // 1969 
        UsedButUnknown3 = 407,
        UsedButUnknown4 = 450,
        UsedButUnknown5 = 451,
        UsedButUnknown6 = 452,
        UsedButUnknown7 = 453,
        CDEType = 501,  // PDOC: Personal Doc | EBOK: ebook | EBSP: ebook sample
        LastUpdateType = 502,
        UpdatedTitle = 503,
        ASINCopy = 504, // I found a copy of ASIN in this record. 
        Language = 524, // <dc:language>
        Alignment = 525, // I found horizontal-lr in this record.
        CreatorBuildNumberCopy = 535,   // I found 1019-d6e4792 in this record, 
                                        // which is a build number of Kindlegen 
                                        // 2.7
        InMemory = 547 // String 'I\x00n\x00M\x00e\x00m\x00o\x00r\x00y\x00' 
                        // found in this record, for KindleGen V2.9 build 
                        // 1029-0897292
    }
}

valued_enum! {
    CreatorSoftware: u32 {
        MobiGen = 1,
        MobipocketCreator = 2,
        KindleGenWindows = 200,
        KindleGenLinux = 201,
        KindleGenMac = 202
    }
}

#[derive(Debug, PartialEq, Hash)]
pub enum ExthTag {
    Contributor(String),
    Language(Language),
    UpdatedTitle(String),
    Author(String),
    Publisher(String),
    ASIN(String),
    Source(String),
    CDEType(String),
    PublishingDate(String), // TODO: Change/parse to a chrono::DateTime
    CreatorSoftware(CreatorSoftware),
    CreatorMajorVersion(u32), 
    CreatorMinorVersion(u32),
    CreatorBuildNumber(u32),
    CoverOffset(u32),
    HasFakeCover(bool),
    ThumbnailOffset(u32),
    KF8CoverURI(String),
    StartReadingAtOffset(u32),
    UsedButUnknown(u32),
    Unhandled { tag_type: ExthType, data: Vec<u8> },
}
impl ExthTag {
    fn read_from(source: &mut Read) -> Result<ExthTag, io::Error> {
        use self::ExthType::*;
        let record_type = ExthType::from(try!(read_u32_be(source)));
        // including type and length fields
        let record_len = try!(read_u32_be(source));
        let data_len = record_len - 8;
        Ok(match record_type {
            Contributor => {
                ExthTag::Contributor(
                    try!(read_string(source, data_len as u64))
                )
            },
            Language => {
                ExthTag::Language(
                    common::Language::from(try!(read_u16_be(source)) as u32)
                )
            },
            UpdatedTitle => {
                ExthTag::UpdatedTitle(
                    try!(read_string(source, data_len as u64))
                )
            },
            Author => {
                ExthTag::Author(
                    try!(read_string(source, data_len as u64))
                )
            },
            Publisher => {
                ExthTag::Publisher(
                    try!(read_string(source, data_len as u64))
                )
            },
            ASIN => {
                ExthTag::ASIN(
                    try!(read_string(source, data_len as u64))
                )
            },
            Source => {
                ExthTag::Source(
                    try!(read_string(source, data_len as u64))
                )
            },
            CDEType => {
                ExthTag::CDEType(
                    try!(read_string(source, data_len as u64))
                )
            }
            PublishingDate => {
                ExthTag::PublishingDate(
                    try!(read_string(source, data_len as u64))
                )
            },
            CreatorSoftware => {
                ExthTag::CreatorSoftware(
                    self::CreatorSoftware::from(
                        try!(read_u32_be(source))
                    )
                )
            },
            CreatorMinorVersion => {
                ExthTag::CreatorMinorVersion(
                    try!(read_u32_be(source))
                )
            },
            CreatorMajorVersion => {
                ExthTag::CreatorMajorVersion(
                    try!(read_u32_be(source))
                )
            },
            CreatorBuildNumber => {
                ExthTag::CreatorBuildNumber(
                    try!(read_u32_be(source))
                )
            },
            CoverOffset => {
                ExthTag::CoverOffset(
                    try!(read_u32_be(source))
                )
            },
            HasFakeCover => {
                ExthTag::HasFakeCover(
                    try!(read_u32_be(source)) == 1
                )
            },
            ThumbnailOffset => {
                ExthTag::ThumbnailOffset(
                    try!(read_u32_be(source))
                )
            },
            KF8CoverURI => {
                ExthTag::KF8CoverURI(
                    try!(read_string(source, data_len as u64))
                )
            },
            StartReadingAtOffset => {
                ExthTag::StartReadingAtOffset(
                    try!(read_u32_be(source))
                )
            },
            UsedButUnknown => {
                ExthTag::UsedButUnknown(
                    try!(read_u32_be(source))
                )
            },
            other_type => {
                let mut data_buf = Vec::new();
                source.take(data_len as u64).read_to_end(&mut data_buf);
                ExthTag::Unhandled { tag_type: other_type, data: data_buf }
            }
        })
    }
}