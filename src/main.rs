#![feature(convert)]
#![feature(plugin)]
#![plugin(regex_macros)]
extern crate regex;

mod lexer;
mod parser;

fn main() {
  let tokens = lexer::tokenize("1 + 2 * (3 - 4);");
  for t in tokens.iter() {
    println!("{:?}", t);
  }
}
