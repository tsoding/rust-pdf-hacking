use std::result;
use std::fs::File;
use std::io::Read;
use flate2::read::ZlibDecoder;

type Result<T> = result::Result<T, ()>;

#[derive(Debug)]
enum Token<'a> {
    Number(i32),
    Symbol(&'a str),
    Dictionary(usize),
    Stream(&'a [u8]),
}

struct PdfParser<'a> {
    content: &'a [u8]
}

impl<'a> PdfParser<'a> {
    fn from_bytes(content: &'a [u8]) -> Self {
        Self { content }
    }

    fn trim_left_spaces(&mut self) {
        let mut index = 0;
        while index < self.content.len() && self.content[index].is_ascii_whitespace() {
            index += 1;
        }
        self.content = &self.content[index..];
    }

    fn drop_line(&mut self) {
        let mut index = 0;
        while index < self.content.len() && self.content[index] != '\n' as u8 {
            index += 1;
        }
        if index < self.content.len() {
            self.content = &self.content[index + 1..]
        } else {
            self.content = &self.content[index..]
        }
    }

    fn trim_left_spaces_and_comments(&mut self) {
        loop {
            self.trim_left_spaces();
            if self.content.len() > 0 && self.content[0] == '%' as u8 {
                self.drop_line();
                continue;
            } else {
                break;
            }
        }
    }

    fn chop_brackets(&mut self, bra: &[u8], ket: &[u8]) -> &[u8] {
        self.content = &self.content[bra.len()..];
        let mut index = 0;
        while index < self.content.len() && !self.content[index..].starts_with(ket) {
            index += 1;
        }
        let bytes = &self.content[0..index];
        if self.content[index..].starts_with(ket) {
            self.content = &self.content[index+ket.len()..];
        } else {
            self.content = &self.content[index..];
        }
        bytes
    }

    fn next_token(&mut self) -> Option<Token> {
        self.trim_left_spaces_and_comments();

        if self.content.len() == 0 {
            return None;
        }

        // Number
        if self.content[0].is_ascii_digit() {
            let mut index = 0;
            while index < self.content.len() && self.content[index].is_ascii_digit() {
                index += 1;
            }
            let number = std::str::from_utf8(&self.content[0..index])
                .expect("sequence of ASCII digits to be a correct UTF-8 string")
                .parse()
                .expect("that the sequence will fit within the limits of i32, but we don't know for sure");
            self.content = &self.content[index..];
            return Some(Token::Number(number));
        }

        // Dictionary
        if self.content.starts_with(b"<<") {
            return Some(Token::Dictionary(self.chop_brackets(b"<<", b">>").len()))
        }

        // Stream
        if self.content.starts_with(b"stream\n") {
            return Some(Token::Stream(self.chop_brackets(b"stream\n", b"\nendstream")));
        }

        // Symbol
        if self.content[0].is_ascii_alphabetic() {
            let mut index = 0;
            while index < self.content.len() && self.content[index].is_ascii_alphanumeric() {
                index += 1;
            }
            let symbol = std::str::from_utf8(&self.content[0..index])
                .expect("sequence of ASCII alphanumerics to be a correct UTF-8 string");
            self.content = &self.content[index..];
            return Some(Token::Symbol(symbol));
        }

        unreachable!("Unknown object")
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args();
    let program = args.next().expect("Program is always provided");
    let file_path = args.next().ok_or_else(|| {
        eprintln!("Usage: {program} <input>");
        eprintln!("ERROR: no input was provided");
    })?;
    let mut content = Vec::new();
    let mut file = File::open(&file_path).map_err(|err| {
        eprintln!("ERROR: could not read file {file_path}: {err}");
    })?;
    file.read_to_end(&mut content).map_err(|err| {
        eprintln!("ERROR: could not read file {file_path}: {err}");
    })?;
    let mut pdf_parser = PdfParser::from_bytes(&content);

    while let Some(token) = pdf_parser.next_token() {
        if let Token::Stream(bytes) = token {
            let mut d = ZlibDecoder::new(bytes);
            let mut s = String::new();
            match d.read_to_string(&mut s) {
                Ok(_) => println!("{s}"),
                Err(err) => {
                    eprintln!("{err}");
                    match std::str::from_utf8(&bytes[0..8]) {
                        Ok(s) => println!("{s}"),
                        Err(err) => eprintln!("{err}"),
                    }
                }
            }
            println!("------------------------------");
        }
    }

    Ok(())
}
