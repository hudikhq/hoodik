use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::error::CryptoResult;
use tokenizers::tokenizer::Tokenizer;

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

/// Use a pre-trained model to convert text into tokens
pub fn into_tokens(input: &str) -> CryptoResult<Vec<Token>> {
    let tokenizer = Tokenizer::from_bytes(include_bytes!("../assets/bert-base-cased.json"))?;
    let input = tokenizers::decoders::wordpiece::cleanup(input).replace(';', "");

    let encoding = tokenizer.encode(input, false)?;
    let encoding = encoding.get_tokens().to_vec();
    let mut map = HashMap::<String, usize>::new();

    for token in encoding {
        let count = map.entry(token.clone()).or_insert(0);
        *count += 1;
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

/// Use a pre-trained model to convert text into hashed tokens
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

        // println!("{:?}", tokens);
        // println!("{:?}", super::into_string(hashed));

        assert_eq!(super::into_string(tokens), "world:1;Hello:1;,:1;!:1");
        assert_eq!(super::into_string(hashed), "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7:1;185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969:1;d03502c43d74a30b936740a9517dc4ea2b2ad7168caa0a774cefe793ce0b33e7:1;bb7208bc9b5d7c04f1236a82a0093a5e33f40423d5ba8d4266f7092c3ba43b62:1");
    }
}
