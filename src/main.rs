#![feature(convert)]
#![feature(plugin)]
#![plugin(regex_macros)]
#![plugin(docopt_macros)]

extern crate regex;
extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use Stage::{AST, Tokens};
use std::io;
use std::io::Write;
use parser::*;
use lexer::*;

mod lexer;
mod parser;

#[derive(PartialEq, Clone, Debug)]
pub enum Stage {
  AST,
  Tokens
}

docopt!(Args, "
Usage: kaleidoscope [(-l | -p | -i)]

Options:
  -l  Run only lexer and show its output.
  -p  Run only parser and show its output.
  -i  Run only IR builder and show its output.
");

fn main() {
  let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

  let stage = if args.flag_l {
    Tokens
  } else {
    AST
  };

  main_loop(stage);
}

fn main_loop(stage: Stage) {
  let stdin = io::stdin();
  let mut stdout = io::stdout();
  let mut input = String::new();
  let mut parser_settings = default_parser_settings();

  let mut ast = Vec::new();
  let mut prev = Vec::new();
  'main: loop {
    print!("> ");
    stdout.flush().unwrap();
    input.clear();
    stdin.read_line(&mut input).ok().expect("Failed to read input");
    if input.as_str() == "quit\n" || input.as_str() == "q\n" {
      break;
    }

    ast.clear();
    prev.clear();
    loop {
      let tokens = tokenize(input.as_str());
      match stage {
        Tokens => {
          println!("{:?}", tokens);
          break;
        },
        AST => {
          prev.extend(tokens.into_iter());

          let parse_result = parse(prev.as_slice(), ast.as_slice(), &mut parser_settings);
          match parse_result {
            Ok((parsed_ast, rest)) => {
              ast.extend(parsed_ast.into_iter());
              if rest.is_empty() {
                break
              } else {
                prev = rest;
              }
            },
            Err(message) => {
              println!("Error: {}", message);
              continue 'main;
            }
          }
          print!(".   ");
          stdout.flush().unwrap();
          input.clear();
          stdin.read_line(&mut input).ok().expect("Failed to read line");
        },
      }
    }
    if stage == AST {
      println!("{:?}", ast);
    }
  }
}
