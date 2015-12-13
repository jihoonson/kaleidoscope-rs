#![feature(convert)]
#![feature(plugin)]
#![plugin(regex_macros)]
extern crate regex;

pub mod lexer;

#[test]
fn it_works() {
  let tokens = lexer::tokenize("1 + 2 * (3 - 4);");
  for t in tokens.iter() {
    println!("{:?}", t);
  }
}
