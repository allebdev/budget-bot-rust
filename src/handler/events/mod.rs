use std::convert::TryFrom;
use std::fmt;

use chrono::NaiveDate;
use regex::Regex;

pub mod google_docs;

pub(crate) type RecordId = i64;

lazy_static! {
    static ref RE_AMOUNT: Regex = Regex::new(r"^-?\d+(?:[.,]\d{1,2})?$").unwrap();
}

#[derive(Debug, PartialEq, Eq)]
pub struct Amount(pub(crate) String);

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for Amount {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if RE_AMOUNT.is_match(value) {
            Ok(Amount(value.replace(',', ".")))
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
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
    fn handle_event(&self, event: HandlerEvent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_integer() {
        assert_eq!(Amount::try_from("42").unwrap().0, "42")
    }

    #[test]
    fn amount_decimal_0_digits_fail() {
        assert!(Amount::try_from("42.").is_err(), "No digits followed dot");
        assert!(Amount::try_from("42,").is_err(), "No digits followed comma");
    }

    #[test]
    fn amount_decimal_1_digit() {
        assert_eq!(Amount::try_from("42.1").unwrap().0, "42.1")
    }

    #[test]
    fn amount_decimal_2_digits() {
        assert_eq!(Amount::try_from("42.13").unwrap().0, "42.13")
    }

    #[test]
    fn amount_decimal_3_digits_fail() {
        assert!(
            Amount::try_from("42.135").is_err(),
            "More than 2 digits followed separator"
        );
    }

    #[test]
    fn amount_decimal_with_comma_replaced() {
        assert_eq!(Amount::try_from("42,13").unwrap().0, "42.13")
    }

    #[test]
    fn amount_negative_is_allowed() {
        assert_eq!(Amount::try_from("-42").unwrap().0, "-42")
    }
}
