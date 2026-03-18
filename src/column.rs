use std::collections::HashMap;

use crate::aesthetic::Aesthetic;
use crate::layout::Unit;

/// Input data columns from CSV / stat transforms — before scale mapping.
#[derive(Clone, Debug)]
pub enum RawColumn {
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),
    StringArray(Vec<String>),
}

impl RawColumn {
    pub fn len(&self) -> usize {
        match self {
            Self::FloatArray(v) => v.len(),
            Self::IntArray(v) => v.len(),
            Self::StringArray(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Try to unpack values as a f64 vector
    pub fn as_f64(&self) -> Result<Vec<f64>, String> {
        match self {
            Self::FloatArray(v) => Ok(v.clone()),
            Self::IntArray(v) => Ok(v.iter().map(|i| *i as f64).collect()),
            Self::StringArray(_) => Err("Cannot convert StringArray to f64".into()),
        }
    }

    /// Select elements at the given indices, preserving order.
    pub fn select_indices(&self, indices: &[usize]) -> Self {
        match self {
            Self::FloatArray(v) => Self::FloatArray(indices.iter().map(|&i| v[i]).collect()),
            Self::IntArray(v) => Self::IntArray(indices.iter().map(|&i| v[i]).collect()),
            Self::StringArray(v) => Self::StringArray(indices.iter().map(|&i| v[i].clone()).collect()),
        }
    }

    /// Return distinct string values in appearance order. Panics if not a StringArray.
    pub fn distinct_strings(&self) -> Vec<String> {
        match self {
            Self::StringArray(v) => {
                let mut seen = Vec::new();
                for s in v {
                    if !seen.contains(s) {
                        seen.push(s.clone());
                    }
                }
                seen
            }
            _ => panic!("distinct_strings called on non-StringArray"),
        }
    }

    /// Return indices where this column's string value equals `value`. Panics if not a StringArray.
    pub fn indices_where_eq(&self, value: &str) -> Vec<usize> {
        match self {
            Self::StringArray(v) => v.iter().enumerate().filter(|(_, s)| s.as_str() == value).map(|(i, _)| i).collect(),
            _ => panic!("indices_where_eq called on non-StringArray"),
        }
    }
}

/// Output of scale mapping — ready for geometry rendering.
#[derive(Clone, Debug)]
pub enum MappedColumn {
    UnitArray(Vec<Unit>),
    ColorArray(Vec<[f32; 3]>),
}

impl MappedColumn {
    pub fn len(&self) -> usize {
        match self {
            Self::UnitArray(v) => v.len(),
            Self::ColorArray(v) => v.len(),
        }
    }
}

/// Aesthetic-keyed raw data (after column renaming, before scale mapping).
#[derive(Clone)]
pub struct AesData {
    data: HashMap<Aesthetic, RawColumn>,
}

impl AesData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get(&self, aes: Aesthetic) -> Option<&RawColumn> {
        self.data.get(&aes)
    }

    pub fn insert(&mut self, aes: Aesthetic, col: RawColumn) {
        self.data.insert(aes, col);
    }

    pub fn contains(&self, aes: Aesthetic) -> bool {
        self.data.contains_key(&aes)
    }
}

/// Fully resolved data for geometry rendering.
pub struct ResolvedData {
    /// Scale-mapped aesthetics (X, Y, Color)
    pub mapped: HashMap<Aesthetic, MappedColumn>,
    /// Unscaled aesthetics (Group)
    pub raw: HashMap<Aesthetic, RawColumn>,
}

/// Raw column data keyed by column name — used at the CSV boundary and passed into Blueprint::render().
#[derive(Clone)]
pub struct PlotData {
    data: HashMap<String, RawColumn>,
}

impl PlotData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn insert(&mut self, key: String, value: RawColumn) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&RawColumn> {
        self.data.get(key)
    }

    /// Create a new PlotData with only the rows at the given indices.
    pub fn subset(&self, indices: &[usize]) -> Self {
        let mut result = Self::new();
        for (key, col) in &self.data {
            result.insert(key.clone(), col.select_indices(indices));
        }
        result
    }
}
