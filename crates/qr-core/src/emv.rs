use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// EMV Parsing Errors
#[derive(Error, Debug, PartialEq)]
pub enum EmvError {
    #[error("Invalid CRC: expected {expected}, got {actual}")]
    InvalidCrc { expected: String, actual: String },
    #[error("Missing Checksum (Tag 63)")]
    MissingChecksum,
    #[error("Parse error: {0}")]
    ParseVerify(String),
    #[error("Malformed TLV data")]
    MalformedData,
}

/// Parsed EMV Data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmvData {
    pub raw_data: String,
    pub pfi: String, // Payload Format Indicator (00)
    pub point_of_initiation: Option<String>, // (01)
    pub merchant_account_information: HashMap<String, String>, // (02-51)
    pub merchant_category_code: Option<String>, // (52)
    pub transaction_currency: Option<String>, // (53)
    pub transaction_amount: Option<String>, // (54)
    pub country_code: Option<String>, // (58)
    pub merchant_name: Option<String>, // (59)
    pub merchant_city: Option<String>, // (60)
    pub postal_code: Option<String>, // (61)
    pub additional_data: HashMap<String, String>, // (62)
    pub crc: String, // (63)
    pub unparsed_tags: HashMap<String, String>, 
}

impl EmvData {
    pub fn parse(raw: &str) -> Result<Self, EmvError> {
        // 1. Validate CRC first
        Self::validate_crc(raw)?;

        let mut tags = HashMap::new();
        let mut idx = 0;
        let query_chars: Vec<char> = raw.chars().collect();
        let len = query_chars.len();

        // 2. Parse TLV
        while idx < len {
            if idx + 4 > len {
                break; // Should ideally be error if trailing garbage, but robust to ignore
            }
            
            let tag: String = query_chars[idx..idx+2].iter().collect();
            let len_str: String = query_chars[idx+2..idx+4].iter().collect();
            
            let value_len = len_str.parse::<usize>().map_err(|_| EmvError::MalformedData)?;
            
            if idx + 4 + value_len > len {
                return Err(EmvError::MalformedData);
            }
            
            let value: String = query_chars[idx+4..idx+4+value_len].iter().collect();
            
            tags.insert(tag, value);
            idx = idx + 4 + value_len;
        }

        // 3. Map to Struct
        let pfi = tags.remove("00").ok_or(EmvError::MalformedData)?;
        let crc = tags.remove("63").ok_or(EmvError::MissingChecksum)?; // Should be present due to step 1
        
        let mut merchant_account_information = HashMap::new();
        let mut additional_data = HashMap::new();
        
        // Extract ranges
        let keys: Vec<String> = tags.keys().cloned().collect();
        for k in keys {
            if let Ok(id) = k.parse::<u32>() {
                if id >= 2 && id <= 51 {
                    if let Some(v) = tags.remove(&k) {
                        merchant_account_information.insert(k, v);
                    }
                } else if id == 62 {
                     if let Some(v) = tags.remove(&k) {
                        // Sub-parsing could go here
                        additional_data.insert(k, v);
                    }
                }
            }
        }

        Ok(EmvData {
            raw_data: raw.to_string(),
            pfi,
            point_of_initiation: tags.remove("01"),
            merchant_account_information,
            merchant_category_code: tags.remove("52"),
            transaction_currency: tags.remove("53"),
            transaction_amount: tags.remove("54"),
            country_code: tags.remove("58"),
            merchant_name: tags.remove("59"),
            merchant_city: tags.remove("60"),
            postal_code: tags.remove("61"),
            additional_data,
            crc,
            unparsed_tags: tags,
        })
    }

    fn validate_crc(raw: &str) -> Result<(), EmvError> {
        let len = raw.len();
        if len < 4 {
             return Err(EmvError::MalformedData);
        }
        
        // Check if last 4 chars are valid hex (they are the checksum)
        // AND the tag 63 + len 04 precedes them.
        // Format: ... + '63' + '04' + 'CRC'
        
        if len < 8 {
             return Err(EmvError::MalformedData);
        }
        
        let checksum_tag = &raw[len-8..len-4]; // Should be '6304'
        if checksum_tag != "6304" {
            // It's possible custom extensions follow, but standard says CRC is last.
            // For robustness, we search for '6304' from the end? 
            // Most specs say CRC is *the last data object*.
            return Err(EmvError::MissingChecksum);
        }
        
        let provided_crc = &raw[len-4..];
        let data_to_check = &raw[..len-4];
        
        let calculated_crc = crc16_ccitt_kermit(data_to_check.as_bytes());
        let calculated_hex = format!("{:04X}", calculated_crc);
        
        if provided_crc.to_uppercase() != calculated_hex {
            return Err(EmvError::InvalidCrc { 
                expected: calculated_hex, 
                actual: provided_crc.to_string() 
            });
        }
        
        Ok(())
    }
}

// CRC-16/CCITT-FALSE (Kermit)
// Poly: 0x1021
// Init: 0xFFFF
fn crc16_ccitt_kermit(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        // crc = (crc >> 8) | (crc << 8); // No, standard CCITT implementation
        let x = ((crc >> 8) ^ (byte as u16)) & 0xFF;
        let mut x = x ^ (x >> 4);
        crc = (crc << 8) ^ (x << 12) ^ (x << 5) ^ x;
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emv() {
        // Sample with rudimentary CRC (Needs real sample or robust test generation)
        // 000201 - PFI 01
        // 5909SomeMerch - Merchant Name
        // 6304.... - CRC
        
        let payload_body = "0002015909SomeMerch6304";
        let crc = crc16_ccitt_kermit(payload_body.as_bytes());
        let full_payload = format!("{}{:04X}", payload_body, crc);
        
        let parsed = EmvData::parse(&full_payload).expect("Should parse");
        assert_eq!(parsed.pfi, "01");
        assert_eq!(parsed.merchant_name, Some("SomeMerch".to_string()));
    }
}
