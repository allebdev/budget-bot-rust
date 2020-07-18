use crate::handler::events::Amount;
use crate::handler::tokenizer::Token::Word;

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Word(&'a str),
    Amount(Amount),
    TrailingSigns(&'a str),
}

pub type MessageTokens<'a> = Vec<Token<'a>>;

const TRAILING_SIGNS: &[char] = &['.', ',', ':', ';', '!', '?'];

pub fn tokenize(text: &str) -> MessageTokens {
    let mut result = Vec::new();
    for word in text.split_whitespace() {
        let original_word = word;
        let word = word.trim_end_matches(TRAILING_SIGNS);
        match word.parse() {
            Ok(amount) => result.push(Token::Amount(amount)),
            Err(..) => result.push(Word(word)),
        }
        if original_word != word {
            result.push(Token::TrailingSigns(&original_word[word.len()..]))
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_word() {
        assert_eq!(tokenize("text"), vec![Token::Word("text")])
    }

    #[test]
    fn multiple_words() {
        assert_eq!(
            tokenize("Some Words"),
            vec![Token::Word("Some"), Token::Word("Words")]
        )
    }

    #[test]
    fn single_amount() {
        assert_eq!(
            tokenize("-42.35"),
            vec![Token::Amount(Amount("-42.35".to_string()))]
        )
    }

    #[test]
    fn words_with_some_numbers() {
        assert_eq!(
            tokenize("7.45 an apple and 2 bananas"),
            vec![
                Token::Amount(Amount("7.45".to_string())),
                Token::Word("an"),
                Token::Word("apple"),
                Token::Word("and"),
                Token::Amount(Amount("2".to_string())),
                Token::Word("bananas"),
            ]
        )
    }

    #[test]
    fn trailing_signs() {
        assert_eq!(
            tokenize("one, two"),
            vec![
                Token::Word("one"),
                Token::TrailingSigns(","),
                Token::Word("two"),
            ]
        );
        assert_eq!(
            tokenize("banana 3,50."),
            vec![
                Token::Word("banana"),
                Token::Amount(Amount("3.50".to_string())),
                Token::TrailingSigns("."),
            ]
        );
    }
}
