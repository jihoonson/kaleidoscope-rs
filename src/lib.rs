#![feature(convert)]
#![feature(plugin)]
#![feature(box_syntax)]

extern crate regex;
extern crate llvm_sys;
extern crate iron_llvm;

pub mod lexer;
pub mod context;
pub mod builder;
pub mod module;
pub mod parser;
pub mod driver;
pub mod jitter;

#[test]
fn it_works() {
  let tokens = lexer::tokenize("1 + 2 * (3 - 4);");
  for t in tokens.iter() {
    println!("{:?}", t);
  }
}
