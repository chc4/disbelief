//! This example s-expr parser is taken from nom's examples (MIT license)
//! In this example we build an [S-expression](https://en.wikipedia.org/wiki/S-expression)
//! parser and tiny [lisp](https://en.wikipedia.org/wiki/Lisp_(programming_language)) interpreter.
//! Lisp is a simple type of language made up of Atoms and Lists, forming easily parsable trees.

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nom::{
  branch::alt,
  bytes::complete::tag,
  character::complete::{alpha1, char, digit1, multispace0, multispace1, one_of},
  combinator::{cut, map, map_res, opt},
  error::{context, VerboseError},
  multi::many0,
  sequence::{delimited, preceded, terminated, tuple},
  IResult, Parser,
};
use std::fmt::Display;
use enum_display_derive::Display;

/// We start by defining the types that define the shape of data that we want.
/// In this case, we want something tree-like

/// Starting from the most basic, we define some built-in functions that our lisp has
#[derive(Debug, PartialEq, Clone, Copy, Display)]
pub enum BuiltIn {
  Plus,
  Minus,
  Times,
  Divide,
  Equal,
  Not,
  Let,
  Set,
  Get,
  Do,
  Loop,
  Resume
}

/// We now wrap this type and a few other primitives into our Atom type.
/// Remember from before that Atoms form one half of our language.
use std::num::Wrapping;
pub type Int = Wrapping<usize>;

#[derive(Debug, PartialEq, Clone)]
pub enum Atom {
  Num(Int),
  Keyword(String),
  //Boolean(bool),
  BuiltIn(BuiltIn),
  Unit,
}

use core::fmt::Formatter;
impl Display for Atom {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Atom::Num(n) => write!(fmt, "{}", n),
            Atom::Keyword(k) => write!(fmt, ":{}", k),
            Atom::BuiltIn(b) => write!(fmt, "{}", b),
            Atom::Unit => write!(fmt, "()"),
        }
    }
}

/// The remaining half is Lists. We implement these as recursive Expressions.
/// For a list of numbers, we have `'(1 2 3)`, which we'll parse to:
/// ```
/// use disbelief::parser::{Expr, Atom};
/// use std::num::Wrapping;
/// Expr::Quote(vec![Expr::Constant(Atom::Num(Wrapping(1))),
///                  Expr::Constant(Atom::Num(Wrapping(2))),
///                  Expr::Constant(Atom::Num(Wrapping(3)))]);
/// ```
/// Quote takes an S-expression and prevents evaluation of it, making it a data
/// structure that we can deal with programmatically. Thus any valid expression
/// is also a valid data structure in Lisp itself.

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
  Constant(Atom),
  /// (func-name arg1 arg2)
  Application(Box<Expr>, Vec<Expr>),
  /// (if predicate do-this)
  //If(Box<Expr>, Box<Expr>),
  /// (if predicate do-this otherwise-do-this)
  IfElse(Box<Expr>, Box<Expr>, Box<Expr>),
  /// '(3 (if (+ 3 3) 4 5) 7)
  Quote(Vec<Expr>),
  Suspend,
  /// (resume coro val)
  Resume(Box<Expr>, Box<Expr>),
  /// (handle :effect val coro)
  Handle(Box<Expr>, Box<Expr>, Box<Expr>),
  /// (raise :effect)
  Raise(Box<Expr>),
}
//impl Expr {
//    pub fn need_int(&mut self) -> Result<Int, crate::EvalError> {
//        match self {
//            Expr::Constant(Atom::Num(u)) => Ok(*u),
//            v => Err(crate::EvalError::TypeError("int", v._type()))
//        }
//    }
//
//    pub fn need_keyword(&mut self) -> Result<String, crate::EvalError> {
//        match self {
//            Expr::Constant(Atom::Keyword(u)) => Ok(u.to_string()),
//            v => Err(crate::EvalError::TypeError("keyword", v._type()))
//        }
//    }
//
//    pub fn _type(&self) -> &'static str {
//        match self {
//            Expr::Constant(Atom::Num(_)) => "int",
//            Expr::Constant(Atom::BuiltIn(_)) => "builtin",
//            _ => "unknown"
//        }
//    }
//}



#[derive(Debug)]
pub struct ListExpr(pub Vec<Expr>);
impl Display for ListExpr {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut first = true;
        for elem in &self.0 {
            if first {
                write!(fmt, "{}", elem)?;
                first = false;
            } else {
                write!(fmt, " {}", elem)?;
            }
        }
        Ok(())
    }
}

impl Display for Expr {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Expr::Constant(a) => write!(fmt, "{}", a),
            Expr::Application(func, body) => write!(fmt, "({} {})", func, ListExpr(body.clone())),
            //Expr::If(cond, body) => write!(fmt, "(if {} {})", cond, body),
            Expr::IfElse(cond, t_body, f_body) => write!(fmt, "(if-else {} {} {})",
                cond, t_body, f_body),
            Expr::Suspend => write!(fmt, "suspend"),
            Expr::Raise(eff) => write!(fmt, "(raise {})", eff),
            Expr::Resume(coro, cont) => write!(fmt, "(resume {} {})", coro, cont),
            Expr::Handle(eff, res, coro) => write!(fmt, "(handle {} {} {})", eff, res, coro),
            Expr::Quote(contents) => write!(fmt, "'({})", ListExpr(contents.clone()))
        }
    }
}

/// Continuing the trend of starting from the simplest piece and building up,
/// we start by creating a parser for the built-in operator functions.
fn parse_builtin_op<'a>(i: &'a str) -> IResult<&'a str, BuiltIn, VerboseError<&'a str>> {
  // one_of matches one of the characters we give it
  map(alt((tag("+"), tag("-"), tag("*"), tag("/"), tag("="),
    tag("let"), tag("set"), tag("get"), tag("do"), tag("loop"))), |stem: &str|
    // because we are matching single character tokens, we can do the matching logic
    // on the returned value
    match stem {
      "+" => BuiltIn::Plus,
      "-" => BuiltIn::Minus,
      "*" => BuiltIn::Times,
      "/" => BuiltIn::Divide,
      "=" => BuiltIn::Equal,
      "let" => BuiltIn::Let,
      "set" => BuiltIn::Set,
      "get" => BuiltIn::Get,
      "do" => BuiltIn::Do,
      "loop" => BuiltIn::Loop,
      _ => unreachable!(),
    })(i)
}

fn parse_builtin<'a>(i: &'a str) -> IResult<&'a str, BuiltIn, VerboseError<&'a str>> {
  // alt gives us the result of first parser that succeeds, of the series of
  // parsers we give it
  alt((
    parse_builtin_op,
    // map lets us process the parsed output, in this case we know what we parsed,
    // so we ignore the input and return the BuiltIn directly
    map(tag("not"), |_| BuiltIn::Not),
  ))(i)
}

/// Our boolean values are also constant, so we can do it the same way
fn parse_bool<'a>(i: &'a str) -> IResult<&'a str, Atom, VerboseError<&'a str>> {
  alt((
    map(tag("#t"), |_| Atom::Num(Wrapping(1))),
    map(tag("#f"), |_| Atom::Num(Wrapping(0))),
  ))(i)
}

/// The next easiest thing to parse are keywords.
/// We introduce some error handling combinators: `context` for human readable errors
/// and `cut` to prevent back-tracking.
///
/// Put plainly: `preceded(tag(":"), cut(alpha1))` means that once we see the `:`
/// character, we have to see one or more alphabetic chararcters or the input is invalid.
fn parse_keyword<'a>(i: &'a str) -> IResult<&'a str, Atom, VerboseError<&'a str>> {
  map(
    context("keyword", preceded(tag(":"), cut(alpha1))),
    |sym_str: &str| Atom::Keyword(sym_str.to_string()),
  )(i)
}

/// Next up is number parsing. We're keeping it simple here by accepting any number (> 1)
/// of digits but ending the program if it doesn't fit into an i32.
fn parse_num<'a>(i: &'a str) -> IResult<&'a str, Atom, VerboseError<&'a str>> {
    map_res(digit1, |digit_str: &str| {
      digit_str.parse::<usize>().map(Wrapping).map(Atom::Num)
    })(i)
}

/// Now we take all these simple parsers and connect them.
/// We can now parse half of our language!
fn parse_atom<'a>(i: &'a str) -> IResult<&'a str, Atom, VerboseError<&'a str>> {
  alt((
    parse_num,
    parse_bool,
    map(parse_builtin, Atom::BuiltIn),
    parse_keyword,
  ))(i)
}

/// We then add the Expr layer on top
fn parse_constant<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
  alt((
    map(parse_atom, |atom| Expr::Constant(atom)),
    parse_suspend,
    parse_raise,
  ))(i)
}

/// Before continuing, we need a helper function to parse lists.
/// A list starts with `(` and ends with a matching `)`.
/// By putting whitespace and newline parsing here, we can avoid having to worry about it
/// in much of the rest of the parser.
///
/// Unlike the previous functions, this function doesn't take or consume input, instead it
/// takes a parsing function and returns a new parsing function.
fn s_exp<'a, O1, F>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O1, VerboseError<&'a str>>
where
  F: Parser<&'a str, O1, VerboseError<&'a str>>,
{
  delimited(
    char('('),
    preceded(multispace0, inner),
    context("closing paren", cut(preceded(multispace0, char(')')))),
  )
}

/// We can now use our new combinator to define the rest of the `Expr`s.
///
/// Starting with function application, we can see how the parser mirrors our data
/// definitions: our definition is `Application(Box<Expr>, Vec<Expr>)`, so we know
/// that we need to parse an expression and then parse 0 or more expressions, all
/// wrapped in an S-expression.
///
/// `tuple` is used to sequence parsers together, so we can translate this directly
/// and then map over it to transform the output into an `Expr::Application`
fn parse_application<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
  let application_inner = map(tuple((parse_expr, many0(parse_expr))), |(head, tail)| {
    Expr::Application(Box::new(head), tail)
  });
  // finally, we wrap it in an s-expression
  s_exp(application_inner)(i)
}

/// Because `Expr::If` and `Expr::IfElse` are so similar (we easily could have
/// defined `Expr::If` to have an `Option` for the else block), we parse both
/// in a single function.
///
/// In fact, we define our parser as if `Expr::If` was defined with an Option in it,
/// we have the `opt` combinator which fits very nicely here.
fn parse_if<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
  let if_inner = context(
    "if expression",
    map(
      preceded(
        // here to avoid ambiguity with other names starting with `if`, if we added
        // variables to our language, we say that if must be terminated by at least
        // one whitespace character
        terminated(tag("if"), multispace1),
        cut(tuple((parse_expr, parse_expr, parse_expr))),
      ),
      |(pred, true_branch, false_branch)| {
          Expr::IfElse(
            Box::new(pred),
            Box::new(true_branch),
            Box::new(false_branch),
            )
      },
    ),
  );
  s_exp(if_inner)(i)
}

fn parse_suspend<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
    map(context("suspend", tag("suspend")),
    |_| Expr::Suspend)(i)
}

fn parse_raise<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
    context("raise expression", map(
        preceded(
            terminated(tag("raise"), multispace1),
            cut(parse_expr),
        ),
    |eff| Expr::Raise(Box::new(eff)))
    )(i)
}

fn parse_resume<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
    s_exp(context("resume expression", map(
        preceded(
            terminated(tag("resume"), multispace1),
            cut(tuple((parse_expr, parse_expr))),
        ),
    |(coro, cont)| Expr::Resume(Box::new(coro), Box::new(cont)))
    ))(i)
}

fn parse_handle<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
    s_exp(context("handle expression", map(
        preceded(
            terminated(tag("handle"), multispace1),
            cut(tuple((parse_expr, parse_expr, parse_expr))),
        ),
    |(eff, res, coro)| Expr::Handle(Box::new(eff), Box::new(res), Box::new(coro)))
    ))(i)
}

/// A quoted S-expression is list data structure.
///
/// This example doesn't have the symbol atom, but by adding variables and changing
/// the definition of quote to not always be around an S-expression, we'd get them
/// naturally.
fn parse_quote<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
  // this should look very straight-forward after all we've done:
  // we find the `'` (quote) character, use cut to say that we're unambiguously
  // looking for an s-expression of 0 or more expressions, and then parse them
  map(
    context("quote", preceded(tag("'"), cut(s_exp(many0(parse_expr))))),
    |exprs| Expr::Quote(exprs),
  )(i)
}

/// We tie them all together again, making a top-level expression parser!

pub fn parse_expr<'a>(i: &'a str) -> IResult<&'a str, Expr, VerboseError<&'a str>> {
  preceded(
    multispace0,
    alt((parse_constant, parse_application, parse_if, parse_quote, parse_resume, parse_handle)),
  )(i)
}
