#![feature(convert)]
#![feature(plugin)]
// #![plugin(regex_macros)]
// #![plugin(docopt_macros)]

extern crate regex;
extern crate docopt;
extern crate rustc_serialize;
extern crate kaleidoscope;
extern crate llvm_sys;
extern crate iron_llvm;

use docopt::Docopt;
use kaleidoscope::driver;

const USAGE: &'static str = "
Usage: kaleidoscope [(-l | -p | -i)]

Options:
  -l  Run only lexer and show its output.
  -p  Run only parser and show its output.
  -i  Run only IR builder and show its output.
";

#[derive(Debug, RustcDecodable)]
struct Args {
  flag_l: bool,
  flag_p: bool,
  flag_i: bool,
}

fn main() {
  // let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
  let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

  let stage = if args.flag_l {
    driver::Tokens
  } else if args.flag_i {
    driver::IR
  } else {
    driver::AST
  };

  driver::main_loop(stage);
}
