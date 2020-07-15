use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use serde::export::Formatter;

use crate::handler::tokenizer::{tokenize, MessageTokens, Token};

#[cfg(test)]
mod tests;

pub trait CategoryProvider {
    fn categories(&self) -> Vec<Category>;
}

pub struct Categorizer {
    categories: Option<BTreeSet<Category>>,
}

impl Categorizer {
    pub(crate) fn new() -> Self {
        Categorizer { categories: None }
    }

    #[allow(dead_code)]
    pub(crate) fn classify_msg(&self, text: &str) -> Option<&Category> {
        self.classify(&tokenize(text))
    }

    pub(crate) fn classify(&self, tokens: &MessageTokens) -> Option<&Category> {
        let categories = self
            .categories
            .as_ref()
            .expect("categories must be loaded before classify text");

        let mut results = BinaryHeap::new();
        for token in tokens {
            if let Token::Word(word) = token {
                for c in categories.iter() {
                    if c.match_word(word) {
                        results.push(c);
                    }
                }
            }
        }

        results.pop().or_else(|| self.default_category())
    }

    pub(crate) fn load_categories<P: CategoryProvider>(&mut self, provider: &P) {
        for c in provider.categories() {
            self.add_category(c);
        }
    }

    fn add_category(&mut self, category: Category) -> bool {
        match &mut self.categories {
            None => {
                self.categories = Some(BTreeSet::from_iter(vec![category]));
                true
            }
            Some(c) => c.insert(category),
        }
    }

    fn default_category(&self) -> Option<&Category> {
        self.categories.as_ref().and_then(|c| c.range(..).next())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    pub name: String,
    priority: i32,
    #[serde(with = "serde_with::rust::display_fromstr")]
    lexemes: LexemeList,
}

impl PartialEq for Category {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.name == other.name
    }
}

impl Eq for Category {}

impl PartialOrd for Category {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for Category {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then(other.name.cmp(&self.name))
    }
}

impl Category {
    #[allow(dead_code)]
    pub fn new(name: String, priority: i32, lexemes: LexemeList) -> Self {
        Category {
            name,
            priority,
            lexemes,
        }
    }
    fn match_word(&self, word: &str) -> bool {
        let word = word.trim().to_lowercase();
        self.lexemes.0.iter().any(|l| word.starts_with(&l.0))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
struct Lexeme(String);

impl From<&str> for Lexeme {
    fn from(text: &str) -> Self {
        Lexeme(text.trim().to_lowercase())
    }
}

impl fmt::Display for Lexeme {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct LexemeList(Vec<Lexeme>);

impl From<&str> for LexemeList {
    fn from(text: &str) -> Self {
        LexemeList(text.split(',').map(Lexeme::from).collect())
    }
}

impl FromStr for LexemeList {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(From::<&str>::from(s))
    }
}

impl fmt::Display for LexemeList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str_vec: Vec<_> = self.0.iter().map(|x| x.0.as_str()).collect();
        write!(f, "{}", str_vec.join(","))
    }
}
