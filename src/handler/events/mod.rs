use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;
use regex::Regex;

#[cfg(feature = "csv-storage")]
use crate::handler::events::csv::CsvEventHandler;
#[cfg(feature = "gss-storage")]
use crate::handler::events::google_docs::GoogleDocsEventHandler;

#[cfg(feature = "csv-storage")]
mod csv;
#[cfg(feature = "gss-storage")]
mod google_docs;

#[cfg(feature = "csv-storage")]
pub type DefaultEventHandler = CsvEventHandler;
#[cfg(feature = "gss-storage")]
pub type DefaultEventHandler = GoogleDocsEventHandler;

pub(crate) type RecordId = i64;

lazy_static! {
    static ref RE_AMOUNT: Regex = Regex::new(r"^-?\d+(?:[.,]\d{1,2})?$").unwrap();
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Amount(pub(crate) String);

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Amount {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if RE_AMOUNT.is_match(s) {
            Ok(Amount(s.replace(',', ".")))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BudgetRecord {
    pub id: RecordId,
    pub date: NaiveDate,
    pub category: String,
    pub amount: Amount,
    pub desc: String,
    pub user: String,
}

#[derive(Debug)]
pub enum HandlerEvent {
    AddRecord(BudgetRecord),
    UpdateRecord(BudgetRecord),
}

pub trait EventHandler {
    fn handle_event(&mut self, event: HandlerEvent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_integer() {
        assert_eq!(Amount::from_str("42").unwrap().0, "42")
    }

    #[test]
    fn amount_decimal_0_digits_fail() {
        assert!(Amount::from_str("42.").is_err(), "No digits followed dot");
        assert!(Amount::from_str("42,").is_err(), "No digits followed comma");
    }

    #[test]
    fn amount_decimal_1_digit() {
        assert_eq!(Amount::from_str("42.1").unwrap().0, "42.1")
    }

    #[test]
    fn amount_decimal_2_digits() {
        assert_eq!(Amount::from_str("42.13").unwrap().0, "42.13")
    }

    #[test]
    fn amount_decimal_3_digits_fail() {
        assert!(
            Amount::from_str("42.135").is_err(),
            "More than 2 digits followed separator"
        );
    }

    #[test]
    fn amount_decimal_with_comma_replaced() {
        assert_eq!(Amount::from_str("42,13").unwrap().0, "42.13")
    }

    #[test]
    fn amount_negative_is_allowed() {
        assert_eq!(Amount::from_str("-42").unwrap().0, "-42")
    }
}
