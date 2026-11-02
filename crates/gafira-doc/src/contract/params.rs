//! Parameter schemas and values of effects, transitions and export options
//! (02 §4.17, ARQ-110).

use std::collections::BTreeMap;

use gafira_color::Color;

/// One value of a parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Choice(String),
    Color(Color),
}

/// Values by parameter id, iterated in id order (ARQ-111).
///
/// Plain data: nothing is validated here (ARQ-110); see
/// [`ParamSchema::check_values`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParamValues {
    values: BTreeMap<String, ParamValue>,
}

impl ParamValues {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `value` under `id` and returns the value it replaced.
    pub fn set(&mut self, id: impl Into<String>, value: ParamValue) -> Option<ParamValue> {
        self.values.insert(id.into(), value)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ParamValue> {
        self.values.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// The value as `f64`; `None` when absent or not a `Float`. No conversion.
    #[must_use]
    pub fn float(&self, id: &str) -> Option<f64> {
        match self.get(id) {
            Some(ParamValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// The value as `i64`; `None` when absent or not an `Int`. No conversion.
    #[must_use]
    pub fn int(&self, id: &str) -> Option<i64> {
        match self.get(id) {
            Some(ParamValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn bool(&self, id: &str) -> Option<bool> {
        match self.get(id) {
            Some(ParamValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    #[must_use]
    pub fn choice(&self, id: &str) -> Option<&str> {
        match self.get(id) {
            Some(ParamValue::Choice(v)) => Some(v),
            _ => None,
        }
    }

    #[must_use]
    pub fn color(&self, id: &str) -> Option<Color> {
        match self.get(id) {
            Some(ParamValue::Color(v)) => Some(*v),
            _ => None,
        }
    }
}
