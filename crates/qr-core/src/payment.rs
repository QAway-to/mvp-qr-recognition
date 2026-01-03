//! Модуль парсинга платёжных форматов QR-кодов
//!
//! Поддерживаемые форматы:
//! - EMV QR Code (международный стандарт)
//! - СБП (Система быстрых платежей, Россия)
//! - ST.00012 (Стандарт ЦБ РФ)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Платёжный формат
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentFormat {
    EmvQR,
    SbpRussia,
    StRussia,
    Unknown,
}

/// Платёжная информация
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    /// Формат платёжного QR
    pub format: PaymentFormat,
    /// Название получателя
    pub payee_name: Option<String>,
    /// Идентификатор получателя (ИНН, merchant ID и т.д.)
    pub payee_id: Option<String>,
    /// Счёт/номер карты
    pub account: Option<String>,
    /// Банк получателя
    pub bank: Option<String>,
    /// БИК банка
    pub bic: Option<String>,
    /// Сумма платежа
    pub amount: Option<f64>,
    /// Валюта (ISO 4217)
    pub currency: Option<String>,
    /// Назначение платежа
    pub purpose: Option<String>,
    /// Дополнительные поля
    pub extra: HashMap<String, String>,
}

impl Default for PaymentInfo {
    fn default() -> Self {
        Self {
            format: PaymentFormat::Unknown,
            payee_name: None,
            payee_id: None,
            account: None,
            bank: None,
            bic: None,
            amount: None,
            currency: None,
            purpose: None,
            extra: HashMap::new(),
        }
    }
}

/// Парсер платёжных QR-кодов
pub struct PaymentParser;

impl Default for PaymentParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentParser {
    /// Создание парсера
    pub fn new() -> Self {
        Self
    }
    
    /// Парсинг платёжного QR
    pub fn parse(&self, content: &str) -> Option<PaymentInfo> {
        // Определяем формат
        if content.starts_with("https://qr.nspk.ru") || content.starts_with("http://qr.nspk.ru") {
            return self.parse_sbp(content);
        }
        
        if content.starts_with("ST.") || content.starts_with("st.") {
            return self.parse_st(content);
        }
        
        if content.starts_with("00") && content.len() > 50 {
            return self.parse_emv(content);
        }
        
        None
    }
    
    /// Оценка релевантности для платежа (0.0 - 1.0)
    pub fn relevance_score(&self, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        
        // Высший приоритет - платёжные URL
        if content_lower.contains("qr.nspk.ru") {
            return 1.0;
        }
        
        // EMV QR
        if content.starts_with("00") && content.len() > 50 {
            return 0.95;
        }
        
        // Российский стандарт
        if content_lower.starts_with("st.") {
            return 0.9;
        }
        
        // Ключевые слова платежей
        let payment_keywords = ["pay", "payment", "оплат", "платёж", "платеж", "перевод"];
        for keyword in &payment_keywords {
            if content_lower.contains(keyword) {
                return 0.6;
            }
        }
        
        // Банковские URL
        if content_lower.contains("bank") || content_lower.contains("банк") {
            return 0.4;
        }
        
        0.0
    }
    
    /// Парсинг СБП QR (НСПК)
    fn parse_sbp(&self, content: &str) -> Option<PaymentInfo> {
        let mut info = PaymentInfo {
            format: PaymentFormat::SbpRussia,
            currency: Some("RUB".to_string()),
            ..Default::default()
        };
        
        // Парсим URL параметры
        // Пример: https://qr.nspk.ru/AS1234567890?type=02&bank=100000000001&sum=10000&cur=RUB&crc=XXXX
        
        if let Some(query_start) = content.find('?') {
            let query = &content[query_start + 1..];
            
            for param in query.split('&') {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[..eq_pos];
                    let value = &param[eq_pos + 1..];
                    
                    match key.to_lowercase().as_str() {
                        "sum" => {
                            // Сумма в копейках
                            if let Ok(kopeks) = value.parse::<f64>() {
                                info.amount = Some(kopeks / 100.0);
                            }
                        }
                        "cur" => {
                            info.currency = Some(value.to_string());
                        }
                        "bank" => {
                            info.bank = Some(value.to_string());
                        }
                        "name" => {
                            info.payee_name = Some(urlencoding::decode(value).unwrap_or_default().to_string());
                        }
                        "purpose" => {
                            info.purpose = Some(urlencoding::decode(value).unwrap_or_default().to_string());
                        }
                        _ => {
                            info.extra.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        }
        
        // Извлекаем идентификатор из пути
        // https://qr.nspk.ru/AS1234567890 -> AS1234567890
        if let Some(path_start) = content.find("nspk.ru/") {
            let path = &content[path_start + 8..];
            let id_end = path.find('?').unwrap_or(path.len());
            info.payee_id = Some(path[..id_end].to_string());
        }
        
        Some(info)
    }
    
    /// Парсинг ST.00012 (Стандарт ЦБ РФ)
    fn parse_st(&self, content: &str) -> Option<PaymentInfo> {
        let mut info = PaymentInfo {
            format: PaymentFormat::StRussia,
            currency: Some("RUB".to_string()),
            ..Default::default()
        };
        
        // Формат: ST.00012|Name=Имя|PersonalAcc=40817...|BankName=...|BIC=...
        for part in content.split('|') {
            if let Some(eq_pos) = part.find('=') {
                let key = &part[..eq_pos];
                let value = &part[eq_pos + 1..];
                
                match key {
                    "Name" => info.payee_name = Some(value.to_string()),
                    "PersonalAcc" => info.account = Some(value.to_string()),
                    "BankName" => info.bank = Some(value.to_string()),
                    "BIC" => info.bic = Some(value.to_string()),
                    "Sum" => {
                        if let Ok(kopeks) = value.parse::<f64>() {
                            info.amount = Some(kopeks / 100.0);
                        }
                    }
                    "Purpose" => info.purpose = Some(value.to_string()),
                    "PayeeINN" => info.payee_id = Some(value.to_string()),
                    _ => {
                        info.extra.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
        
        Some(info)
    }
    
    /// Парсинг EMV QR Code
    fn parse_emv(&self, content: &str) -> Option<PaymentInfo> {
        let mut info = PaymentInfo {
            format: PaymentFormat::EmvQR,
            ..Default::default()
        };
        
        // EMV использует TLV (Tag-Length-Value) формат
        // Каждое поле: 2 цифры тег + 2 цифры длина + значение
        
        let mut pos = 0;
        let bytes = content.as_bytes();
        
        while pos + 4 <= bytes.len() {
            // Тег (2 цифры)
            let tag = std::str::from_utf8(&bytes[pos..pos + 2]).ok()?;
            pos += 2;
            
            // Длина (2 цифры)
            let len_str = std::str::from_utf8(&bytes[pos..pos + 2]).ok()?;
            let len: usize = len_str.parse().ok()?;
            pos += 2;
            
            if pos + len > bytes.len() {
                break;
            }
            
            // Значение
            let value = std::str::from_utf8(&bytes[pos..pos + len]).ok()?;
            pos += len;
            
            match tag {
                "00" => {} // Payload Format Indicator
                "01" => {} // Point of Initiation Method
                "52" => {
                    // Merchant Category Code
                    info.extra.insert("mcc".to_string(), value.to_string());
                }
                "53" => {
                    // Transaction Currency (ISO 4217 numeric)
                    info.currency = Some(self.currency_code_to_string(value));
                }
                "54" => {
                    // Transaction Amount
                    info.amount = value.parse().ok();
                }
                "58" => {
                    // Country Code
                    info.extra.insert("country".to_string(), value.to_string());
                }
                "59" => {
                    // Merchant Name
                    info.payee_name = Some(value.to_string());
                }
                "60" => {
                    // Merchant City
                    info.extra.insert("city".to_string(), value.to_string());
                }
                _ => {
                    // Сохраняем остальные теги
                    if tag.starts_with("26") || tag.starts_with("27") {
                        // Merchant Account Information
                        info.account = Some(value.to_string());
                    }
                }
            }
        }
        
        Some(info)
    }
    
    /// Конвертация числового кода валюты в строку
    fn currency_code_to_string(&self, code: &str) -> String {
        match code {
            "643" => "RUB".to_string(),
            "840" => "USD".to_string(),
            "978" => "EUR".to_string(),
            "156" => "CNY".to_string(),
            "392" => "JPY".to_string(),
            "826" => "GBP".to_string(),
            _ => code.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sbp_parsing() {
        let parser = PaymentParser::new();
        let content = "https://qr.nspk.ru/AS10001234567890ABCDEF?type=02&bank=100000000001&sum=10000&cur=RUB";
        
        let result = parser.parse(content).unwrap();
        assert_eq!(result.format, PaymentFormat::SbpRussia);
        assert_eq!(result.amount, Some(100.0)); // 10000 копеек = 100 рублей
        assert_eq!(result.currency, Some("RUB".to_string()));
    }
    
    #[test]
    fn test_st_parsing() {
        let parser = PaymentParser::new();
        let content = "ST.00012|Name=ООО Тест|PersonalAcc=40817810099910004312|BIC=044525225|Sum=100000";
        
        let result = parser.parse(content).unwrap();
        assert_eq!(result.format, PaymentFormat::StRussia);
        assert_eq!(result.payee_name, Some("ООО Тест".to_string()));
        assert_eq!(result.amount, Some(1000.0)); // 100000 копеек = 1000 рублей
    }
    
    #[test]
    fn test_relevance_score() {
        let parser = PaymentParser::new();
        
        assert_eq!(parser.relevance_score("https://qr.nspk.ru/test"), 1.0);
        assert!(parser.relevance_score("Hello World") < 0.1);
        assert!(parser.relevance_score("Оплата заказа") > 0.5);
    }
}
