use std::fmt::Write;

use askama::filters::Escaper;

#[derive(Clone, Copy)]
pub struct Conf;

impl Escaper for Conf {
  fn write_escaped_str<W>(&self, mut fmt: W, string: &str) -> std::fmt::Result
  where
    W: Write,
  {
    fmt.write_str(string)
  }
}

#[derive(Clone, Copy)]
pub struct Js;

impl Escaper for Js {
  fn write_escaped_str<W>(&self, mut fmt: W, string: &str) -> std::fmt::Result
  where
    W: Write,
  {
    fmt.write_str(string)
  }
}
