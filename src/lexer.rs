use regex::Regex;

pub use self::Token::{
  Def,
  Extern,
  If,
  Then,
  Else,
  For,
  In,
  Delimiter,
  LeftParen,
  RightParen,
  Comma,
  Binary,
  Unary,
  Ident,
  Number,
  Operator
};

#[derive(PartialEq, Clone, Debug)]
pub enum Token {
  Def,
  Extern,
  If,
  Then,
  Else,
  For,
  In,
  Delimiter,
  LeftParen,
  RightParen,
  Comma,
  Binary,
  Unary,
  Ident(String),
  Number(f64),
  Operator(String)
}

pub fn tokenize(input: &str) -> Vec<Token> {
  // let comment_re = regex!(r"(?m)#.*\n");
  let comment_re = Regex::new(r"(?m)#.*\n").unwrap();
  let preprocessed = comment_re.replace_all(input, "\n");

  // let token_re = regex!(concat!(
  //         r"(?P<ident>\p{Alphabetic}\w*)|",
  //         r"(?P<number>\d+\.?\d*)|",
  //         r"(?P<delimiter>;)|",
  //         r"(?P<oppar>\()|",
  //         r"(?P<clpar>\))|",
  //         r"(?P<comma>,)|",
  //         r"(?P<operator>\S)"));
  let token_re = Regex::new(concat!(
          r"(?P<ident>\p{Alphabetic}\w*)|",
          r"(?P<number>\d+\.?\d*)|",
          r"(?P<delimiter>;)|",
          r"(?P<oppar>\()|",
          r"(?P<clpar>\))|",
          r"(?P<comma>,)|",
          r"(?P<operator>\S)")).unwrap();

  let result: Vec<Token> = token_re.captures_iter(preprocessed.as_str()).map(|cap| {
    match vec!["ident", "number", "delimiter", "oppar", "clpar", "comma", "operator"].iter()
      .find(|keyword| cap.name(keyword).is_some()) {
        None => panic!("Undefined token"),
        Some(k) => {
          match *k {
            "ident" => {
              match cap.name(k).unwrap() {
                "def" => Def,
                "extern" => Extern,
                "if" => If,
                "then" => Then,
                "else" => Else,
                "for" => For,
                "in" => In,
                "binary" => Binary,
                "unary" => Unary,
                ident => Ident(ident.to_string()),
              }
            },
            "number" => {
              match cap.name("number").unwrap().parse() {
                  Ok(number) => Number(number),
                  Err(_) => panic!("Lexer failed trying to parse number")
              }
            },
            "delimiter" => Delimiter,
            "oppar" => LeftParen,
            "clpar" => RightParen,
            "comma" => Comma,
            "operator" => Operator(cap.name("operator").unwrap().to_string()),
            _ => panic!("Undefined token: {}", *k)
          }
        }
      }
  }).collect::<Vec<_>>();

  result
}
