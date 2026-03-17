use std::cell::RefCell;
use std::collections::HashMap;

use itertools::Itertools as _;
use lazy_static::lazy_static;
use postgres_types::ToSql;
use regex::{Regex, Replacer};
use thiserror::Error;

thread_local! {
  static MAPPING_CACHE: RefCell<HashMap<String, (String, ParameterPlacer)>> = Default::default();
}

lazy_static! {
  static ref PARAMETER_REGEX: Regex = Regex::new("\\?([a-zA-Z0-9_]+)").expect("Invalid regex.");
}

#[derive(Debug, Default)]
pub struct QueryBuilder<'a> {
  fragments: Vec<QueryFragment<'a>>,
}

#[derive(Default)]
pub struct ParameterPlacer {
  source: HashMap<String, usize>,
  value_mapping: Vec<usize>,
  last_error: Option<ParameterPlacerError>,
}

#[derive(Clone, Debug, Error)]
pub enum ParameterPlacerError {
  #[error("Missing a parameter: {0}")]
  MissingParameter(String),
  #[error("CaptureMissing")]
  CaptureMissing,
}

impl ParameterPlacer {
  fn new(value_map: ValueMap<'_, '_>) -> ParameterPlacer {
    let source = value_map
      .iter()
      .enumerate()
      .map(|(idx, (name, _))| (name.to_string(), idx))
      .collect::<HashMap<_, _>>();

    ParameterPlacer {
      source,
      ..Default::default()
    }
  }

  fn to_values<'a, 'b>(&self, value_map: ValueMap<'a, 'b>) -> Values<'a> {
    self
      .value_mapping
      .iter()
      .map(|idx| value_map[*idx].1)
      .collect::<Values<'a>>()
  }

  fn validate(&mut self) -> Result<(), ParameterPlacerError> {
    match self.last_error.take() {
      Some(err) => Err(err),
      None => Ok(()),
    }
  }
}

impl Replacer for &mut ParameterPlacer {
  fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
    match caps.get(1).map(|cap| cap.as_str()) {
      Some(cap) => match self.source.get(cap).copied() {
        Some(value_idx) => self.value_mapping.push(value_idx),
        None => self.last_error = Some(ParameterPlacerError::MissingParameter(cap.to_owned())),
      },
      None => self.last_error = Some(ParameterPlacerError::CaptureMissing),
    }

    dst.push('?');
  }
}

#[derive(Clone, Debug, Error)]
pub enum QueryBuilderError {
  #[error(transparent)]
  ParameterPlacerError(#[from] ParameterPlacerError),
}

impl<'a> QueryBuilder<'a> {
  pub fn fragment<'b: 'a, F>(mut self, fragment: F) -> Self
  where
    F: Into<Fragment<'b>>,
  {
    let Fragment(query, values) = fragment.into();

    self.fragments.push(QueryFragment { query, values });

    self
  }

  pub fn conditional<'b: 'a, T, C, F>(self, condition: bool, callback: C) -> Self
  where
    C: FnOnce() -> F,
    F: Into<Fragment<'b>>,
  {
    match condition {
      true => self.fragment(callback()),
      false => self,
    }
  }

  pub fn optional<'b: 'a, T, C, F>(self, option: Option<T>, callback: C) -> Self
  where
    C: FnOnce(T) -> F,
    F: Into<Fragment<'b>>,
  {
    match option.map(callback) {
      Some(value) => self.fragment(value),
      None => self,
    }
  }

  pub fn parameters<'b: 'a, 'c>(mut self, query: &str, values: ValuesSlice<'a, 'c>) -> Self {
    self.fragments.push(QueryFragment {
      query: query.to_owned(),
      values: values.into(),
    });

    self
  }

  fn _parameters_mapped(
    query: &str,
    value_map: ValueMap<'a, '_>,
  ) -> Result<(String, ParameterPlacer), QueryBuilderError> {
    let mut placer = ParameterPlacer::new(value_map);

    let query = PARAMETER_REGEX.replace_all(query, &mut placer).to_string();

    placer.validate()?;

    Ok((query, placer))
  }

  pub fn parameters_mapped(
    mut self,
    query: &str,
    value_map: ValueMap<'a, '_>,
  ) -> Result<Self, QueryBuilderError> {
    let (query, placer) = Self::_parameters_mapped(query, value_map)?;

    let values = placer.to_values(value_map);

    self.fragments.push(QueryFragment { query, values });

    Ok(self)
  }

  pub fn parameters_mapped_cached(
    mut self,
    cache_key: &str,
    query: &str,
    value_map: ValueMap<'a, '_>,
  ) -> Result<Self, QueryBuilderError> {
    if let Some((query, values)) = MAPPING_CACHE.with_borrow(|mapping| {
      mapping
        .get(cache_key)
        .map(|(query, placer)| (query.to_owned(), placer.to_values(value_map)))
    }) {
      self.fragments.push(QueryFragment { query, values });

      return Ok(self);
    }

    let (query, placer) = Self::_parameters_mapped(query, value_map)?;

    let values = placer.to_values(value_map);

    MAPPING_CACHE.with_borrow_mut(|mapping| {
      mapping.insert(cache_key.to_owned(), (query.to_owned(), placer));
    });

    self.fragments.push(QueryFragment { query, values });

    Ok(self)
  }

  pub fn wrap<'b: 'a, F>(self, wrap: &str, fragment: F) -> Self
  where
    F: Into<Fragment<'b>>,
  {
    let Fragment(query, values) = fragment.into();

    self.parameters(&wrap.replace("{}", &query), &values[..])
  }

  pub fn wrap_fn<'b: 'a, F, T>(self, wrap: &str, fragment: F) -> Self
  where
    F: Fn() -> T,
    T: Into<Fragment<'b>>,
  {
    let Fragment(query, values) = fragment().into();

    self.parameters(&wrap.replace("{}", &query), &values[..])
  }

  pub fn parameters_join<'b: 'a>(self, values: &Values<'a>, join: &str) -> Self {
    let fragment = (0..values.len()).map(|_| "?").join(join);

    self.parameters(&fragment, values)
  }

  pub fn parameters_join_cast<'b: 'a>(self, values: &Values<'a>, join: &str, cast: &str) -> Self {
    let fragment = (0..values.len()).map(|_| format!("?::{cast}")).join(join);

    self.parameters(&fragment, values)
  }

  pub fn wrap_present<'b: 'a, F>(self, wrap: &str, fragment: F) -> Self
  where
    F: Into<Fragment<'b>>,
  {
    let Fragment(query, values) = fragment.into();

    if !query.is_empty() {
      return self.parameters(&wrap.replace("{}", &query), &values[..]);
    }

    self
  }

  pub fn join<'b>(mut self, join: &'b str) -> (String, Values<'a>) {
    let (fragments, values) = self.fragments.drain(..).fold(
      (vec![], vec![]),
      |(mut sum_fragments, mut sum_values), QueryFragment { query, values }| {
        sum_fragments.push(query);
        sum_values.extend(values);

        (sum_fragments, sum_values)
      },
    );

    (fragments.join(join), values)
  }

  pub fn build(self) -> (String, Values<'a>) {
    self.join(" ")
  }

  pub fn finish(self) -> (String, Values<'a>) {
    let (mut query, values) = self.join(" ");

    let nplaceholders = query.chars().filter(|c| *c == '?').count();

    for i in 1..=nplaceholders {
      query = query.replacen("?", &format!("${}", &i), 1);
    }

    (query, values)
  }
}

pub struct Fragment<'a>(pub String, pub Values<'a>);

impl<'b> From<&'b str> for Fragment<'_> {
  fn from(fragment: &'b str) -> Self {
    Fragment(fragment.to_owned(), vec![])
  }
}

impl<'a, 'b, 'c> From<(&'b str, ValuesSlice<'a, 'c>)> for Fragment<'a> {
  fn from((query, values): (&'b str, ValuesSlice<'a, 'c>)) -> Self {
    Fragment(query.to_owned(), values.into())
  }
}

impl<'a> From<(String, Values<'a>)> for Fragment<'a> {
  fn from((query, values): (String, Values<'a>)) -> Self {
    Fragment(query, values)
  }
}

impl<'a> From<QueryBuilder<'a>> for Fragment<'a> {
  fn from(from: QueryBuilder<'a>) -> Self {
    let (query, values) = from.build();

    Fragment(query, values)
  }
}

#[derive(Debug)]
pub struct QueryFragment<'a> {
  query: String,
  values: Values<'a>,
}

pub type ValueMap<'a, 'b> = &'b [(&'b str, Value<'a>)];
pub type Values<'a> = Vec<Value<'a>>;
pub type ValuesSlice<'a, 'b> = &'b [Value<'a>];
pub type Value<'a> = &'a (dyn ToSql + Sync);

#[cfg(test)]
mod test {
  use super::QueryBuilder;

  #[test]
  fn test() {
    let (query, values) = QueryBuilder::default()
      .fragment("SELECT")
      .fragment(
        QueryBuilder::default()
          .fragment("a")
          .fragment("b")
          .join(","),
      )
      .fragment("WHERE")
      .fragment(
        QueryBuilder::default()
          .parameters("i = ?", &[&2i32])
          .parameters("j = ?", &[&4i32])
          .join(","),
      )
      .finish();

    println!("query: {}", &query);
    println!("values: {:?}", &values);
  }
}
