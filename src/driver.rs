pub use self::Stage::{Exec, AST, Tokens, IR};
use std::io;
use std::io::Write;

use parser::*;
use lexer::*;
use module::{ModuleProvider, SimpleModuleProvider};
use jitter;
use jitter::JITter;
use context::Context;
use builder;
use builder::IRBuilder;

use llvm_sys::core::LLVMDumpValue;

use iron_llvm::LLVMRef;
use iron_llvm::core::value::Value;
use iron_llvm::target;

#[derive(PartialEq, Clone, Debug)]
pub enum Stage {
  Exec,
  IR,
  AST,
  Tokens
}

pub fn main_loop(stage: Stage) {
  let stdin = io::stdin();
  let mut stdout = io::stdout();
  let mut input = String::new();
  let mut parser_settings = default_parser_settings();
  let mut ir_container: Box<JITter> = match stage {
    Exec => {
      target::initilalize_native_target();
      target::initilalize_native_asm_printer();
      jitter::init();
      Box::new(jitter::MCJITter::new("main"))
    },
    _ => Box::new(SimpleModuleProvider::new("main"))
  };
  let mut builder_context = Context::new();

  let mut ast = Vec::new();
  let mut prev = Vec::new();
  'main: loop {
    print!("> ");
    stdout.flush().unwrap();
    input.clear();
    stdin.read_line(&mut input).ok().expect("Failed to read input");
    if input.as_str() == "quit\n" || input.as_str() == "q\n"
      || input.as_str() == "exit\n" {
      break;
    }

    ast.clear();
    prev.clear();
    loop {
      let tokens = tokenize(input.as_str());
      if stage == Tokens {
        println!("{:?}", tokens);
        continue 'main
      };

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
          continue 'main
        }
      }
      print!(".\t");
      stdout.flush().unwrap();
      input.clear();
      stdin.read_line(&mut input).ok().expect("Failed to read line");
    }

    if stage == AST {
      println!("{:?}", ast);
      continue
    }

    match ast.codegen(&mut builder_context, ir_container.get_module_provider()) {
      Ok((value, runnable)) => {
        if runnable && stage == Exec {
          println!("=> {}", ir_container.run_function(value));
        } else {
          // value.dump()
          unsafe {
              LLVMDumpValue(value.to_ref())
          }
        }
      },
      Err(message) => println!("Error occured: {}", message)
    }
  }

  if stage == IR || stage == Exec {
    ir_container.dump();
  }
}
