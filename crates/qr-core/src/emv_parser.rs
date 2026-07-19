use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmvData {
    pub scheme: Option<String>,
    pub pfi: Option<String>, // Tag 26 (Payment Facilitator ID)
    pub merchant_account: HashMap<String, String>, // Tag 26-51 subfields
    pub merchant_category_code: Option<String>, // Tag 52
    pub currency: Option<String>, // Tag 53
    pub amount: Option<String>, // Tag 54
    pub country: Option<String>, // Tag 58
    pub merchant_name: Option<String>, // Tag 59
    pub merchant_city: Option<String>, // Tag 60
    pub postal_code: Option<String>, // Tag 61
    pub additional_data: Option<String>, // Tag 62
    pub crc: Option<String>, // Tag 63
    
    // Raw parsed tags for flexibility
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct TlvTag {
    pub id: String,
    pub length: usize,
    pub value: String,
}

pub fn parse_emv_qr(payload: &str) -> anyhow::Result<EmvData> {
    let tags = parse_tlv(payload)?;
    
    let mut data = EmvData {
        scheme: tags.get("00").cloned(),
        pfi: None,
        merchant_account: HashMap::new(),
        merchant_category_code: tags.get("52").cloned(),
        currency: tags.get("53").cloned(),
        amount: tags.get("54").cloned(),
        country: tags.get("58").cloned(),
        merchant_name: tags.get("59").cloned(),
        merchant_city: tags.get("60").cloned(),
        postal_code: tags.get("61").cloned(),
        additional_data: tags.get("62").cloned(),
        crc: tags.get("63").cloned(),
        tags: tags.clone(),
    };

    // Sub-parsing for Merchant Account Info (Tags 26-51)
    for i in 26..=51 {
        let tag_id = format!("{:02}", i);
        if let Some(val) = tags.get(&tag_id) {
            data.merchant_account.insert(tag_id, val.clone());
            // TODO: If this tag has sub-tags (nested TLV), parse them here? 
            // For typical SBP/EMV, Tag 26 often contains sub-tags (00=GUID, etc.)
        }
    }

    // Validate CRC if present
    if let Some(crc_str) = &data.crc {
        // The CRC is the last 4 chars. We calculate CRC of everything BEFORE the CRC value itself (but including tag '63' and '04')
        // Format: ... "6304" [CRC]
        // So we need to cut off the last 4 chars of the INPUT payload
        if payload.len() >= 4 {
            let data_to_verify = &payload[..payload.len() - 4];
            let calculated_crc = crc16_ccitt_false(data_to_verify.as_bytes());
            let provided_crc = u16::from_str_radix(crc_str, 16).unwrap_or(0);
            
            if calculated_crc != provided_crc {
                log::warn!("CRC Mismatch: Calc {:04X} vs Prov {:04X}", calculated_crc, provided_crc);
            }
        }
    }

    Ok(data)
}

fn parse_tlv(input: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut tags = HashMap::new();
    let mut chars = input.chars().peekable();
    let mut idx = 0;

    while idx < input.len() {
        // Tag ID (2 chars)
        if idx + 2 > input.len() { break; }
        let id_str = &input[idx..idx+2];
        idx += 2;

        // Length (2 chars)
        if idx + 2 > input.len() { break; }
        let len_str = &input[idx..idx+2];
        let len: usize = len_str.parse().unwrap_or(0);
        idx += 2;

        // Value
        if idx + len > input.len() { break; }
        let val_str = &input[idx..idx+len];
        tags.insert(id_str.to_string(), val_str.to_string());
        idx += len;
    }

    Ok(tags)
}

/// CRC-16/CCITT-FALSE (Polynomial 0x1021, Initial 0xFFFF)
/// Commonly used in EMV QR Codes (SBP)
pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16() {
        // "123456789" -> 0x29B1
        let data = b"123456789";
        let crc = crc16_ccitt_false(data);
        assert_eq!(crc, 0x29B1);
    }

    #[test]
    fn test_tlv_parse() {
        // Example: 00 02 01 01 02 11 (Tag 00, Len 02, Val 01; Tag 01, Len 02, Val 11)
        let payload = "000201010211";
        let tags = parse_tlv(payload).unwrap();
        assert_eq!(tags.get("00").unwrap(), "01");
        assert_eq!(tags.get("01").unwrap(), "11");
    }
}
