//! Parameter schemas and values of effects, transitions and export options
//! (02 §4.17, ARQ-110).

use std::collections::BTreeMap;
use std::fmt;

use gafira_color::Color;

use crate::msg::MessageKey;

/// One option of a [`ParamKind::Choice`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamChoice {
    /// Stable id, the value stored in [`ParamValue::Choice`].
    pub id: &'static str,
    /// Catalog key of the option's label (ARQ-98).
    pub label: MessageKey,
}

/// The type of a parameter with its range and default.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamKind {
    Float {
        min: f64,
        max: f64,
        default: f64,
    },
    Int {
        min: i64,
        max: i64,
        default: i64,
    },
    Bool {
        default: bool,
    },
    Choice {
        options: &'static [ParamChoice],
        default: &'static str,
    },
    Color {
        default: Color,
    },
}

impl ParamKind {
    /// True when bounds, default and options agree with each other. An
    /// inverted range is empty and a finite range holds no NaN or infinity,
    /// so for the numeric kinds the default check covers those faults.
    fn is_consistent(&self) -> bool {
        match self {
            ParamKind::Float { min, max, default } => {
                min.is_finite() && max.is_finite() && (min..=max).contains(&default)
            }
            ParamKind::Int { min, max, default } => (min..=max).contains(&default),
            ParamKind::Bool { .. } => true,
            ParamKind::Choice { options, default } => {
                options.iter().any(|o| o.id == *default)
                    && options
                        .iter()
                        .enumerate()
                        .all(|(i, o)| options[..i].iter().all(|earlier| earlier.id != o.id))
            }
            ParamKind::Color { default } => default.is_finite(),
        }
    }

    fn default_value(&self) -> ParamValue {
        match self {
            ParamKind::Float { default, .. } => ParamValue::Float(*default),
            ParamKind::Int { default, .. } => ParamValue::Int(*default),
            ParamKind::Bool { default } => ParamValue::Bool(*default),
            ParamKind::Choice { default, .. } => ParamValue::Choice((*default).to_owned()),
            ParamKind::Color { default } => ParamValue::Color(*default),
        }
    }
}

/// One parameter: id, label, kind and whether it can be animated.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamDef {
    pub id: &'static str,
    /// Catalog key of the parameter's label (ARQ-98).
    pub label: MessageKey,
    pub kind: ParamKind,
    pub animatable: bool,
}

/// The ordered parameters of an effect, transition or export format.
///
/// Plain data: [`ParamSchema::new`] validates nothing (ARQ-110). Call
/// [`ParamSchema::validate_definition`] on a schema you did not write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParamSchema {
    params: Vec<ParamDef>,
}

impl ParamSchema {
    #[must_use]
    pub fn new(params: Vec<ParamDef>) -> Self {
        Self { params }
    }

    /// The parameters in definition order.
    #[must_use]
    pub fn params(&self) -> &[ParamDef] {
        &self.params
    }

    /// The first parameter with this id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.id == id)
    }

    /// Checks the schema itself, as an engine author wrote it.
    ///
    /// # Errors
    ///
    /// [`ParamError::BadDefinition`] naming the first fault in definition
    /// order: a repeated id, a non-finite bound or default, `min > max`, a
    /// default outside `[min, max]`, a `Choice` default that is not an option
    /// or a repeated option id, or a non-finite `Color` default.
    pub fn validate_definition(&self) -> Result<(), ParamError> {
        let mut seen: Vec<&str> = Vec::with_capacity(self.params.len());
        for p in &self.params {
            if seen.contains(&p.id) || !p.kind.is_consistent() {
                return Err(ParamError::BadDefinition { param: p.id });
            }
            seen.push(p.id);
        }
        Ok(())
    }

    /// Every parameter at its default.
    #[must_use]
    pub fn defaults(&self) -> ParamValues {
        let mut values = ParamValues::new();
        for p in &self.params {
            values.set(p.id, p.kind.default_value());
        }
        values
    }
}

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

/// Why a schema or a set of values was rejected. `param` is the parameter id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamError {
    /// A `Float` or `Color` value is NaN or infinite.
    NonFinite { param: &'static str },
    /// A value is outside `[min, max]` or is not one of the options.
    OutOfRange { param: &'static str },
    /// A value is of another kind than the parameter.
    WrongType { param: &'static str },
    /// The schema itself is inconsistent.
    BadDefinition { param: &'static str },
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamError::NonFinite { param } => write!(f, "parameter `{param}` is not finite"),
            ParamError::OutOfRange { param } => write!(f, "parameter `{param}` is out of range"),
            ParamError::WrongType { param } => write!(f, "parameter `{param}` has the wrong type"),
            ParamError::BadDefinition { param } => {
                write!(f, "parameter `{param}` has an invalid definition")
            }
        }
    }
}

impl std::error::Error for ParamError {}
