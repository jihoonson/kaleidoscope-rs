use lexer::Token;
use lexer::Token::{
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
  Ident,
  Number,
  Operator
};
use std::collections::HashMap;
use parser::PartParsingResult::{Good, NotComplete, Bad};
pub use self::ASTNode::{ExternNode, FunctionNode};
pub use self::Expression::{LiteralExpr, VariableExpr, BinaryExpr, CallExpr, ConditionalExpr, LoopExpr};

#[derive(PartialEq, Clone, Debug)]
pub enum ASTNode {
  ExternNode(Prototype),
  FunctionNode(Function)
}

#[derive(PartialEq, Clone, Debug)]
pub struct Function {
  pub prototype: Prototype,
  pub expression: Expression,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Prototype {
  pub name: String,
  pub args: Vec<String>
}

#[derive(PartialEq, Clone, Debug)]
pub enum Expression {
  LiteralExpr(f64),
  VariableExpr(String),
  BinaryExpr(String, Box<Expression>, Box<Expression>),
  ConditionalExpr{cond_expr: Box<Expression>, then_expr: Box<Expression>, else_expr: Box<Expression>},
  LoopExpr{var_name: String, start_expr: Box<Expression>, end_expr: Box<Expression>, step_expr: Box<Expression>, body_expr: Box<Expression>},
  CallExpr(String, Vec<Expression>)
}

#[derive(PartialEq, Clone, Debug)]
pub struct ParserSettings {
  operator_precedence: HashMap<String, i32>
}

enum PartParsingResult<T> {
  Good(T, Vec<Token>),
  NotComplete,
  Bad(String)
}

pub type ParsingResult = Result<(Vec<ASTNode>, Vec<Token>), String>;

pub fn default_parser_settings() -> ParserSettings {
  let mut operator_precedence = HashMap::new();
  operator_precedence.insert("=".to_string(), 2);
  operator_precedence.insert("<".to_string(), 10);
  operator_precedence.insert("+".to_string(), 20);
  operator_precedence.insert("-".to_string(), 20);
  operator_precedence.insert("*".to_string(), 40);

  ParserSettings{operator_precedence: operator_precedence}
}

pub fn parse(tokens: &[Token], parsed_trees: &[ASTNode], settings: &mut ParserSettings) -> ParsingResult {
  let mut rest = tokens.to_vec();
  rest.reverse();

  let mut asts = parsed_trees.to_vec();

  loop {
    let cur_token = match rest.last() {
      Some(t) => t.clone(),
      None => break
    };

    let result = match cur_token {
      Def => parse_function(&mut rest, settings),
      Extern => parse_extern(&mut rest, settings),
      Delimiter => {rest.pop(); continue},
      _ => parse_expression(&mut rest, settings)
    };

    match result {
      Good(ast_node, _) => asts.push(ast_node),
      NotComplete => break,
      Bad(message) => return Err(message),
    }
  }

  rest.reverse();
  Ok((asts, rest))
}

macro_rules! parse_try(
  ($function: ident, $tokens: ident, $settings: ident, $parsed_tokens: ident) => (
    parse_try!($function, $tokens, $settings, $parsed_tokens, )
  );

  ($function: ident, $tokens: ident, $settings: ident, $parsed_tokens: ident, $($arg: expr),*) => (
    match $function($tokens, $settings, $($arg),*) {
      Good(ast, toks) => {
        $parsed_tokens.extend(toks.into_iter());
        ast
      },
      NotComplete => {
        $parsed_tokens.reverse();
        $tokens.extend($parsed_tokens.into_iter());
        return NotComplete
      },
      Bad(message) => return Bad(message)
    }
  )
);

macro_rules! expect_tokens(
  ([ $($token: pat, $value: expr, $result: stmt);+ ] <= $tokens: ident, $parsed_tokens: ident, $error: expr) => (
    match $tokens.pop() {
      $(
        Some($token) => {
          $parsed_tokens.push($value);
          $result
        },
      )+
      None => {
        $parsed_tokens.reverse();
        $tokens.extend($parsed_tokens.into_iter());
        return NotComplete
      },
      _ => return error($error)
    }
  );

  ([ $($token:pat, $value:expr, $result: stmt);+ ] else $not_matched: block <= $tokens: ident, $parsed_tokens: ident) => (
    match $tokens.last().map(|t| t.clone()) {
      $(
        Some($token) => {
          $tokens.pop();
          $parsed_tokens.push($value);
          $result
        },
      )+
      _ => {$not_matched}
    }
  );
);

fn parse_extern(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<ASTNode> {
  tokens.pop();
  let mut parsed_tokens = vec![Extern];
  let prototype = parse_try!(parse_prototype, tokens, settings, parsed_tokens);
  Good(ExternNode(prototype), parsed_tokens)
}

fn parse_function(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<ASTNode> {
  tokens.pop();
  let mut parsed_tokens = vec![Def];
  let prototype = parse_try!(parse_prototype, tokens, settings, parsed_tokens);
  let expression = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  Good(FunctionNode(Function{prototype: prototype, expression: expression}), parsed_tokens)
}

fn parse_prototype(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Prototype> {
  let mut parsed_tokens = Vec::new();

  let name = expect_tokens!(
    [Ident(name), Ident(name.clone()), name] <=
      tokens, parsed_tokens, "expected function name in prototype"
  );

  expect_tokens!(
    [LeftParen, LeftParen, ()] <=
      tokens, parsed_tokens, "expected '(' in prototype"
  );

  let mut args = Vec::new();
  loop {
    // TODO: need to check
    expect_tokens!(
      [
      Ident(arg), Ident(arg.clone()), args.push(arg.clone());
      Comma, Comma, continue;
      RightParen, RightParen, break
      ] <= tokens, parsed_tokens, "expected ')' in prototype"
    );
  }

  Good(Prototype{name: name, args: args}, parsed_tokens)
}

fn parse_expression(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<ASTNode> {
  // tokens.pop();
  let mut parsed_tokens = Vec::new();
  let expression = parse_try!(parse_expr, tokens, settings, parsed_tokens);
  let prototype = Prototype{name: "".to_string(), args: vec![]};
  let func = Function{prototype: prototype, expression: expression};
  Good(FunctionNode(func), parsed_tokens)
}

fn parse_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  let mut parsed_tokens = Vec::new();
  let lhs = parse_try!(parse_primary_expr, tokens, settings, parsed_tokens);
  let expr = parse_try!(parse_binary_expr, tokens, settings, parsed_tokens, 0, &lhs);
  Good(expr, parsed_tokens)
}

fn parse_primary_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  match tokens.last() {
    Some(&Ident(_)) => parse_ident_expr(tokens, settings),
    Some(&Number(_)) => parse_literal_expr(tokens, settings),
    Some(&If) => parse_conditional_expr(tokens, settings),
    Some(&For) => parse_for_expr(tokens, settings),
    Some(&LeftParen) => parse_paren_expr(tokens, settings),
    None => NotComplete,
    _ => error("unknown token when expecting an expression")
  }
}

fn parse_ident_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  let mut parsed_tokens = Vec::new();

  let name = expect_tokens!(
    [Ident(name), Ident(name.clone()), name] <= tokens, parsed_tokens, "identifier expected"
  );

  expect_tokens!(
    [LeftParen, LeftParen, ()]
    else { return Good(VariableExpr(name), parsed_tokens) }
    <= tokens, parsed_tokens
  );

  let mut args = Vec::new();
  loop {
    // TODO: need to check
    expect_tokens!(
      [RightParen, RightParen, break;
      Comma, Comma, continue]
      else {
        args.push(parse_try!(parse_expr, tokens, settings, parsed_tokens));
      }
      <= tokens, parsed_tokens
    );
  }

  Good(CallExpr(name, args), parsed_tokens)
}

fn parse_literal_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  let mut parsed_tokens = Vec::new();

  let value = expect_tokens!(
    [Number(val), Number(val), val] <= tokens, parsed_tokens, "literal expected"
  );

  Good(LiteralExpr(value), parsed_tokens)
}

fn parse_conditional_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  tokens.pop();
  let mut parsed_tokens = vec![If];
  let cond_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  expect_tokens!(
    [Then, Then, ()] <= tokens,
    parsed_tokens, "expected then");
  let then_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  expect_tokens!(
    [Else, Else, ()] <= tokens,
    parsed_tokens, "expected else");
  let else_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  Good(ConditionalExpr{cond_expr: box cond_expr, then_expr: box then_expr, else_expr: box else_expr}, parsed_tokens)
}

fn parse_for_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  tokens.pop();
  let mut parsed_tokens = vec![For];
  let var_name = expect_tokens!(
    [Ident(name), Ident(name.clone()), name] <= tokens,
    parsed_tokens, "expected identifier after for"
  );

  expect_tokens!(
    [Operator(op), Operator(op.clone()), {
      if op.as_str() != "=" {
        return error("expected '=' after for")
      }
    }] <= tokens,
    parsed_tokens, "expected '=' after for"
  );

  let start_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  expect_tokens!(
    [Comma, Comma, ()] <= tokens,
    parsed_tokens, "expected ',' after for start expression"
  );

  let end_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  let step_expr = expect_tokens!(
    [Comma, Comma, parse_try!(parse_expr, tokens, settings, parsed_tokens)]
    else {LiteralExpr(1.0)}
    <= tokens, parsed_tokens
  );

  expect_tokens!(
    [In, In, ()] <= tokens, parsed_tokens, "expected 'in' after for"
  );

  let body_expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  Good(LoopExpr{var_name: var_name, start_expr: box start_expr, end_expr: box end_expr, step_expr: box step_expr, body_expr: box body_expr}, parsed_tokens)
}

fn parse_paren_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings) -> PartParsingResult<Expression> {
  tokens.pop();
  let mut parsed_tokens = vec![LeftParen];

  let expr = parse_try!(parse_expr, tokens, settings, parsed_tokens);

  let paren = expect_tokens!(
    [RightParen, RightParen, ()] <= tokens, parsed_tokens, "')' expected"
  );

  Good(expr, parsed_tokens)
}

fn parse_binary_expr(tokens: &mut Vec<Token>, settings: &mut ParserSettings, expr_precedence: i32, lhs: &Expression) -> PartParsingResult<Expression> {
  let mut result = lhs.clone();
  let mut parsed_tokens = Vec::new();

  loop {
    let (operator, precedence) = match tokens.last() {
      Some(&Operator(ref op_name)) => match settings.operator_precedence.get(op_name) {
        Some(pr) if *pr >= expr_precedence => (op_name.clone(), *pr),
        None => return error("unknown operator"),
        _ => break
      },
      _ => break
    };

    tokens.pop();
    parsed_tokens.push(Operator(operator.clone()));

    let mut rhs = parse_try!(parse_primary_expr, tokens, settings, parsed_tokens);

    loop {
      let binary_rhs = match tokens.last().map(|next_op| next_op.clone()) {
        Some(Operator(ref op_name)) => match settings.operator_precedence.get(op_name).map(|i| *i) {
          Some(pr) if pr > precedence => {
            parse_try!(parse_binary_expr, tokens, settings, parsed_tokens, pr, &rhs)
          },
          None => return error("unknown operator"),
          _ => break
        },
        _ => break
      };

      rhs = binary_rhs;
    }

    result = BinaryExpr(operator, Box::new(result), Box::new(rhs));
  }

  Good(result, parsed_tokens)
}

fn error<T>(message: &str) -> PartParsingResult<T> {
  Bad(message.to_string())
}
