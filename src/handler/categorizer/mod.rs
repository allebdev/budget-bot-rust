use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};
use std::iter::FromIterator;

#[cfg(test)]
mod tests;

pub struct Categorizer {
    categories: Option<BTreeSet<Category>>,
}

impl Categorizer {
    pub(crate) fn new() -> Self {
        Categorizer { categories: None }
    }

    pub(crate) fn classify(&self, text: &str) -> Option<&Category> {
        let categories = self
            .categories
            .as_ref()
            .expect("categories must be loaded before classify text");

        let mut results = BinaryHeap::new();
        for word in text.split_whitespace() {
            for c in categories.iter() {
                if c.match_word(word) {
                    results.push(c);
                }
            }
        }

        results.pop().or_else(|| self.default_category())
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

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    priority: i32,
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
    fn match_word(&self, word: &str) -> bool {
        let word = word.trim().to_lowercase();
        self.lexemes.0.iter().any(|l| word.starts_with(&l.0))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Lexeme(String);

impl From<&str> for Lexeme {
    fn from(text: &str) -> Self {
        Lexeme(text.trim().to_lowercase())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct LexemeList(Vec<Lexeme>);

impl From<&str> for LexemeList {
    fn from(text: &str) -> Self {
        LexemeList(text.split(',').map(Lexeme::from).collect())
    }
}

pub fn load_categories(categorizer: &mut Categorizer) {
    categorizer.add_category(Category {
        name: String::from("Sweets"),
        priority: 10,
        lexemes: "cand,sweet,chocolate".into(),
    });
    categorizer.add_category(Category {
        name: String::from("Fruits"),
        priority: 20,
        lexemes: "apple,banana,orange".into(),
    });
    categorizer.add_category(Category {
        name: String::from("Others"),
        priority: 99999,
        lexemes: "other,misc".into(),
    });
}
