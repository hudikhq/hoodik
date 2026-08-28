use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::error::CryptoResult;

#[derive(Debug, Clone)]
pub struct Token {
    pub token: String,
    pub weight: usize,
}

impl Token {
    pub fn new(token: String, weight: usize) -> Self {
        Self { token, weight }
    }

    /// Hash the token using sha256 digest
    pub fn hashed(&mut self) -> &mut Self {
        self.token = sha256::digest(self.token.as_bytes());

        self
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.token, self.weight)
    }
}

/// Split text into whole-word tokens with occurrence counts.
///
/// Words are maximal alphanumeric runs, case-folded, one character or
/// shorter dropped. This replaced a BERT wordpiece tokenizer whose subword
/// fragments (`para`, `##de`, `01`…) matched incidentally in every
/// text-rich document: a query for a filename ranked the file itself
/// behind dozens of notes that merely contained fragments of it, and
/// gibberish matched half the drive. Whole words make a tag mean the word
/// it was made from — the HMAC keying hides it from the server either way.
pub fn into_tokens(input: &str) -> CryptoResult<Vec<Token>> {
    let mut map = HashMap::<String, usize>::new();

    for word in input.split(|c: char| !c.is_alphanumeric()) {
        if word.chars().count() < 2 {
            continue;
        }
        *map.entry(word.to_lowercase()).or_insert(0) += 1;
    }

    let mut tokens = vec![];
    for (token, weight) in map.into_iter() {
        tokens.push(Token::new(token, weight));
    }

    tokens.sort_by(|a, b| {
        if a.weight > b.weight {
            std::cmp::Ordering::Less
        } else if a.weight < b.weight {
            std::cmp::Ordering::Greater
        } else if a.token > b.token {
            std::cmp::Ordering::Less
        } else if a.token < b.token {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    Ok(tokens)
}

/// Tokenize text and hash every token.
pub fn into_hashed_tokens(input: &str) -> CryptoResult<Vec<Token>> {
    let mut tokens = into_tokens(input)?;

    for token in tokens.iter_mut() {
        token.hashed();
    }

    Ok(tokens)
}

/// Convert vector of tokes into a string for easy wasm transport
pub fn into_string(tokens: Vec<Token>) -> String {
    tokens
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<String>>()
        .join(";")
}

/// Take vector of strings that might be tokens and convert them into tokens
pub fn from_vec(string_tokens: Vec<String>) -> CryptoResult<Vec<Token>> {
    let mut tokens = vec![];

    for token in string_tokens {
        let mut split = token.split(':');
        let token = match split.next() {
            Some(token) => token,
            None => continue,
        };

        let weight = match split.next() {
            Some(weight) => weight,
            None => continue,
        };

        let weight = match weight.parse::<usize>() {
            Ok(weight) => weight,
            Err(_) => continue,
        };

        tokens.push(Token::new(token.to_string(), weight));
    }

    Ok(tokens)
}

#[cfg(test)]
mod test {

    #[test]
    fn into_tokens() {
        let input = "Hello, world!";

        let tokens = super::into_tokens(input).unwrap();
        let hashed = super::into_hashed_tokens(input).unwrap();

        assert_eq!(super::into_string(tokens), "world:1;hello:1");
        assert_eq!(
            super::into_string(hashed),
            "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7:1;2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824:1"
        );
    }

    /// The regression that motivated whole-word tokens: a filename splits
    /// into its meaningful parts and nothing else, so querying any part of
    /// it matches the file instead of every note containing a fragment.
    #[test]
    fn filenames_tokenize_into_their_parts() {
        let tokens = super::into_tokens("IMG_0179.mov").unwrap();
        let mut words: Vec<&str> = tokens.iter().map(|t| t.token.as_str()).collect();
        words.sort();

        assert_eq!(words, ["0179", "img", "mov"]);
    }

    #[test]
    fn short_fragments_and_punctuation_are_dropped() {
        let tokens = super::into_tokens("a b, c! Ć-x9 šuma").unwrap();
        let mut words: Vec<&str> = tokens.iter().map(|t| t.token.as_str()).collect();
        words.sort();

        assert_eq!(words, ["x9", "šuma"]);
    }

    #[test]
    fn repeated_words_accumulate_weight() {
        let tokens = super::into_tokens("note Note NOTE plan").unwrap();

        assert_eq!(super::into_string(tokens), "note:3;plan:1");
    }
}
