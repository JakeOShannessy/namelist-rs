use std::{
    collections::VecDeque,
    io::{Cursor, Read},
};
use utf8::{self, BufReadDecoder};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub lo: usize,
    pub len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocatedToken {
    pub span: Span,
    pub token: Token,
}

impl LocatedToken {
    pub fn token(&self) -> &Token {
        &self.token
    }
    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    LeftBracket,
    RightBracket,
    Equals,
    Colon,
    Comma,
    RightSlash,
    Ampersand,
    /// Some variable string that forms a token. Currently this could also
    /// include numbers.
    QuotedStr(String),
    Whitespace(String),
    Identifier(String),
    Number(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenizerState {
    Start,
    InQuote { start: usize, content: String },
    InIdentifier { start: usize, content: String },
    InNumber { start: usize, content: String },
    InWhitespace { start: usize, content: String },
}

pub struct CharDecoder<R: std::io::Read> {
    iter: BufReadDecoder<std::io::BufReader<R>>,
    offset: usize,
    chars: VecDeque<(usize, char)>,
}

impl<R: Read> CharDecoder<R> {
    pub fn new(input: R) -> Self {
        Self {
            iter: BufReadDecoder::new(std::io::BufReader::new(input)),
            chars: VecDeque::new(),
            offset: 0,
        }
    }
}

impl<R: Read> Iterator for CharDecoder<R> {
    type Item = Result<(usize, char), ()>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(res) = self.chars.pop_front() {
                return Some(Ok(res));
            } else {
                match self.iter.next_strict()? {
                    Ok(next_string) => {
                        let offset = self.offset;
                        for r in next_string.char_indices().map(|(i, c)| (i + offset, c)) {
                            self.chars.push_back(r);
                        }
                        self.offset += next_string.len();
                    }
                    Err(_e) => return Some(Err(())),
                }
            }
        }
    }
}

pub struct TokenIter<B: std::io::Read> {
    iter: CharDecoder<std::io::BufReader<B>>,
    buf: Option<(usize, char)>,
    state: TokenizerState,
}

impl<R: std::io::Read> TokenIter<R> {
    pub fn new(input: R) -> Self {
        Self {
            iter: CharDecoder::new(std::io::BufReader::new(input)),
            buf: None,
            state: TokenizerState::Start,
        }
    }
}

impl<R: std::io::Read> Iterator for TokenIter<R> {
    type Item = Result<LocatedToken, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((i, c)) = self
                .buf
                .take()
                .or_else(|| self.iter.next().map(|x| x.unwrap()))
            {
                match &mut self.state {
                    TokenizerState::Start => {
                        if c.is_whitespace() {
                            let start = i;
                            let mut content = String::new();
                            content.push(c);
                            self.state = TokenizerState::InWhitespace { start, content };
                        } else {
                            match c {
                                '\'' => {
                                    let start = i;
                                    let mut content = String::new();
                                    content.push(c);
                                    self.state = TokenizerState::InQuote { start, content };
                                }
                                '=' => {
                                    let token = Token::Equals;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                '(' => {
                                    let token = Token::LeftBracket;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                ')' => {
                                    let token = Token::RightBracket;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                ':' => {
                                    let token = Token::Colon;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                ',' => {
                                    let token = Token::Comma;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                '/' => {
                                    let token = Token::RightSlash;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                '&' => {
                                    let token = Token::Ampersand;
                                    let span = Span { lo: i, len: 1 };
                                    let token = LocatedToken { span, token };
                                    self.state = TokenizerState::Start;
                                    break Some(Ok(token));
                                }
                                _ => {
                                    if c.is_alphabetic() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InIdentifier { start, content };
                                    } else if c.is_whitespace() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InWhitespace { start, content };
                                    } else if c.is_digit(10) || c == '-' {
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state = TokenizerState::InNumber { start: i, content };
                                    } else {
                                        panic!("{} is an invalid character", c)
                                    }
                                }
                            }
                        }
                    }
                    TokenizerState::InQuote { start, content } => match c {
                        '\'' => {
                            content.push(c);
                            let len = content.len();
                            let value = std::mem::take(content);
                            let token = LocatedToken {
                                span: Span { lo: *start, len },
                                token: Token::QuotedStr(value),
                            };
                            self.state = TokenizerState::Start;
                            break Some(Ok(token));
                        }
                        _ => {
                            content.push(c);
                        }
                    },
                    TokenizerState::InWhitespace { start, content } => {
                        if c.is_whitespace() {
                            content.push(c);
                        } else {
                            let len = content.len();
                            let value = std::mem::take(content);
                            let token = LocatedToken {
                                span: Span { lo: *start, len },
                                token: Token::Whitespace(value),
                            };
                            match c {
                                '\'' => {
                                    let start = i;
                                    let mut content = String::new();
                                    content.push(c);
                                    self.state = TokenizerState::InQuote { start, content };
                                }
                                '=' | '(' | ')' | ':' | ',' | '/' | '&' => {
                                    self.buf.replace((i, c));
                                    self.state = TokenizerState::Start;
                                }
                                _ => {
                                    if c.is_alphabetic() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InIdentifier { start, content };
                                    } else if c.is_whitespace() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InWhitespace { start, content };
                                    } else if c.is_digit(10)
                                        || c == '.'
                                        || c == 'e'
                                        || c == 'E'
                                        || c == '-'
                                        || c == '+'
                                    {
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state = TokenizerState::InNumber { start: i, content };
                                    } else {
                                        panic!("{} is an invalid character", c)
                                    }
                                }
                            }
                            break Some(Ok(token));
                        }
                    }
                    TokenizerState::InIdentifier { start, content } => {
                        if c.is_alphanumeric() || c == '_' {
                            content.push(c);
                        } else {
                            let len = content.len();
                            let value = std::mem::take(content);
                            let span = Span { lo: *start, len };
                            let token = Token::Identifier(value);
                            self.buf.replace((i, c));
                            self.state = TokenizerState::Start;
                            let token = LocatedToken { span, token };
                            break Some(Ok(token));
                        }
                    }
                    TokenizerState::InNumber { start, content } => {
                        if c.is_digit(10) || c == '.' || c == 'e' || c == '-' {
                            content.push(c);
                        } else {
                            let len = content.len();
                            let value = std::mem::take(content);
                            let token = LocatedToken {
                                span: Span { lo: *start, len },
                                token: Token::Number(value),
                            };
                            match c {
                                '\'' => {
                                    let start = i;
                                    let mut content = String::new();
                                    content.push(c);
                                    self.state = TokenizerState::InQuote { start, content };
                                }
                                '=' | '(' | ')' | ':' | ',' | '/' | '&' => {
                                    self.buf.replace((i, c));
                                    self.state = TokenizerState::Start;
                                }
                                _ => {
                                    if c.is_alphabetic() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InIdentifier { start, content };
                                    } else if c.is_whitespace() {
                                        let start = i;
                                        let mut content = String::new();
                                        content.push(c);
                                        self.state =
                                            TokenizerState::InWhitespace { start, content };
                                    } else {
                                        panic!("{} is an invalid character", c);
                                    }
                                }
                            }
                            break Some(Ok(token));
                        }
                    }
                }
            } else {
                match &mut self.state {
                    TokenizerState::Start => {
                        break None;
                    }
                    TokenizerState::InQuote { .. } => {
                        panic!("Unclosed quoted string")
                    }
                    TokenizerState::InWhitespace { start, content } => {
                        let len = content.len();
                        let value = std::mem::take(content);
                        let token = Token::Whitespace(value);
                        let span = Span { lo: *start, len };
                        self.state = TokenizerState::Start;
                        let token = LocatedToken { span, token };
                        break Some(Ok(token));
                    }
                    TokenizerState::InIdentifier { start, content } => {
                        let len = content.len();
                        let value = std::mem::take(content);
                        let token = Token::Identifier(value);
                        let span = Span { lo: *start, len };
                        self.state = TokenizerState::Start;
                        let token = LocatedToken { span, token };
                        break Some(Ok(token));
                    }
                    TokenizerState::InNumber { start, content } => {
                        let len = content.len();
                        let value = std::mem::take(content);
                        let token = Token::Number(value);
                        let span = Span { lo: *start, len };
                        self.state = TokenizerState::Start;
                        let token = LocatedToken { span, token };
                        break Some(Ok(token));
                    }
                }
            }
        }
    }
}

pub fn tokenize_reader<R: Read>(input: R) -> Vec<LocatedToken> {
    let iter = TokenIter::new(input).map(|x| x.unwrap());
    iter.collect()
}

pub fn tokenize_str(input: &str) -> Vec<LocatedToken> {
    let input = Cursor::new(input);
    let iter = TokenIter::new(input).map(|x| x.unwrap());
    iter.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_tokens1() {
        let s = "abc=2";
        let tokens = tokenize_str(s);
        assert_eq!(
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 3 },
                    token: Token::Identifier("abc".to_string()),
                },
                LocatedToken {
                    span: Span { lo: 3, len: 1 },
                    token: Token::Equals,
                },
                LocatedToken {
                    span: Span { lo: 4, len: 1 },
                    token: Token::Number("2".to_string()),
                }
            ],
            tokens
        );
    }

    #[test]
    fn trivial_tokens2() {
        let s = "abc= 2";
        let tokens = tokenize_str(s);
        assert_eq!(
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 3 },
                    token: Token::Identifier("abc".to_string()),
                },
                LocatedToken {
                    span: Span { lo: 3, len: 1 },
                    token: Token::Equals,
                },
                LocatedToken {
                    span: Span { lo: 4, len: 1 },
                    token: Token::Whitespace(" ".to_string()),
                },
                LocatedToken {
                    span: Span { lo: 5, len: 1 },
                    token: Token::Number("2".to_string()),
                }
            ],
            tokens
        );
    }

    #[test]
    fn trivial_tokens3() {
        assert_eq!(
            tokenize_str(")=2"),
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 1 },
                    token: Token::RightBracket
                },
                LocatedToken {
                    span: Span { lo: 1, len: 1 },
                    token: Token::Equals
                },
                LocatedToken {
                    span: Span { lo: 2, len: 1 },
                    token: Token::Number("2".to_string())
                },
            ]
        );
    }

    #[test]
    fn trivial_tokens4() {
        let s = "&abc=2/";
        let tokens = tokenize_str(s);
        assert_eq!(
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 1 },
                    token: Token::Ampersand,
                },
                LocatedToken {
                    span: Span { lo: 1, len: 3 },
                    token: Token::Identifier("abc".to_string()),
                },
                LocatedToken {
                    span: Span { lo: 4, len: 1 },
                    token: Token::Equals,
                },
                LocatedToken {
                    span: Span { lo: 5, len: 1 },
                    token: Token::Number("2".to_string()),
                },
                LocatedToken {
                    span: Span { lo: 6, len: 1 },
                    token: Token::RightSlash,
                },
            ],
            tokens
        );
    }

    #[test]
    fn simple_tokens1() {
        let s = "abc=2,'ad c' (2,:)";
        let tokens = tokenize_str(s);
        let expected = vec![
            LocatedToken {
                span: Span { lo: 0, len: 3 },
                token: Token::Identifier("abc".to_string()),
            },
            LocatedToken {
                span: Span { lo: 3, len: 1 },
                token: Token::Equals,
            },
            LocatedToken {
                span: Span { lo: 4, len: 1 },
                token: Token::Number("2".to_string()),
            },
            LocatedToken {
                span: Span { lo: 5, len: 1 },
                token: Token::Comma,
            },
            LocatedToken {
                span: Span { lo: 6, len: 6 },
                token: Token::QuotedStr("'ad c'".to_string()),
            },
            LocatedToken {
                span: Span { lo: 12, len: 1 },
                token: Token::Whitespace(" ".to_string()),
            },
            LocatedToken {
                span: Span { lo: 13, len: 1 },
                token: Token::LeftBracket,
            },
            LocatedToken {
                span: Span { lo: 14, len: 1 },
                token: Token::Number("2".to_string()),
            },
            LocatedToken {
                span: Span { lo: 15, len: 1 },
                token: Token::Comma,
            },
            LocatedToken {
                span: Span { lo: 16, len: 1 },
                token: Token::Colon,
            },
            LocatedToken {
                span: Span { lo: 17, len: 1 },
                token: Token::RightBracket,
            },
        ];
        assert_eq!(expected, tokens);
    }

    #[test]
    fn simple_tokens2() {
        assert_eq!(
            tokenize_str("TEMPERATURES(1:2)=273.15, 274"),
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 12 },
                    token: Token::Identifier("TEMPERATURES".to_string())
                },
                LocatedToken {
                    span: Span { lo: 12, len: 1 },
                    token: Token::LeftBracket
                },
                LocatedToken {
                    span: Span { lo: 13, len: 1 },
                    token: Token::Number("1".to_string())
                },
                LocatedToken {
                    span: Span { lo: 14, len: 1 },
                    token: Token::Colon
                },
                LocatedToken {
                    span: Span { lo: 15, len: 1 },
                    token: Token::Number("2".to_string())
                },
                LocatedToken {
                    span: Span { lo: 16, len: 1 },
                    token: Token::RightBracket
                },
                LocatedToken {
                    span: Span { lo: 17, len: 1 },
                    token: Token::Equals
                },
                LocatedToken {
                    span: Span { lo: 18, len: 6 },
                    token: Token::Number("273.15".to_string())
                },
                LocatedToken {
                    span: Span { lo: 24, len: 1 },
                    token: Token::Comma
                },
                LocatedToken {
                    span: Span { lo: 25, len: 1 },
                    token: Token::Whitespace(" ".to_string())
                },
                LocatedToken {
                    span: Span { lo: 26, len: 3 },
                    token: Token::Number("274".to_string())
                }
            ]
        );
    }

    #[test]
    fn simple_tokens3() {
        assert_eq!(
            tokenize_str("TEMPERATURES(1:2)=273.15, \n 274"),
            vec![
                LocatedToken {
                    span: Span { lo: 0, len: 12 },
                    token: Token::Identifier("TEMPERATURES".to_string())
                },
                LocatedToken {
                    span: Span { lo: 12, len: 1 },
                    token: Token::LeftBracket
                },
                LocatedToken {
                    span: Span { lo: 13, len: 1 },
                    token: Token::Number("1".to_string())
                },
                LocatedToken {
                    span: Span { lo: 14, len: 1 },
                    token: Token::Colon
                },
                LocatedToken {
                    span: Span { lo: 15, len: 1 },
                    token: Token::Number("2".to_string())
                },
                LocatedToken {
                    span: Span { lo: 16, len: 1 },
                    token: Token::RightBracket
                },
                LocatedToken {
                    span: Span { lo: 17, len: 1 },
                    token: Token::Equals
                },
                LocatedToken {
                    span: Span { lo: 18, len: 6 },
                    token: Token::Number("273.15".to_string())
                },
                LocatedToken {
                    span: Span { lo: 24, len: 1 },
                    token: Token::Comma
                },
                LocatedToken {
                    span: Span { lo: 25, len: 3 },
                    token: Token::Whitespace(" \n ".to_string())
                },
                LocatedToken {
                    span: Span { lo: 28, len: 3 },
                    token: Token::Number("274".to_string())
                }
            ]
        );
    }
}
