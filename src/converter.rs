//! Abstract converter trait for WVG documents.
//!
//! This module defines the `Converter` trait that allows conversion of WVG
//! documents to various output formats. Implementations can target SVG, PNG,
//! or any other format.

use crate::error::WvgResult;
use crate::types::WvgDocument;

/// A trait for converting WVG documents to other formats.
///
/// Implementations of this trait can convert a parsed `WvgDocument` into
/// various output formats such as SVG, PNG, or custom formats.
///
/// # Type Parameter
///
/// * `Output` - The type of the conversion output (e.g., `String` for SVG,
///   `Vec<u8>` for binary formats).
///
/// # Example
///
/// ```ignore
/// use wvg::{Converter, WvgDocument};
///
/// struct MyConverter;
///
/// impl Converter for MyConverter {
///     type Output = String;
///
///     fn convert(&self, document: &WvgDocument) -> WvgResult<Self::Output> {
///         // Convert the document to your desired format
///         Ok(String::new())
///     }
/// }
/// ```
pub trait Converter {
    /// The output type of the conversion.
    type Output;

    /// Converts the given WVG document to the output format.
    ///
    /// # Arguments
    ///
    /// * `document` - The parsed WVG document to convert.
    ///
    /// # Returns
    ///
    /// Returns the converted output on success, or an error if conversion fails.
    fn convert(&self, document: &WvgDocument) -> WvgResult<Self::Output>;
}

/// Configuration options for converters.
///
/// This struct provides common configuration that may be used by various
/// converter implementations.
#[derive(Debug, Clone, Default)]
pub struct ConverterConfig {
    /// Whether to include comments in the output (if supported).
    pub include_comments: bool,

    /// Whether to pretty-print/format the output (if applicable).
    pub pretty_print: bool,

    /// Custom line width multiplier.
    pub line_width_scale: Option<f32>,
}

impl ConverterConfig {
    /// Creates a new converter configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to include comments in the output.
    pub fn with_comments(mut self, include: bool) -> Self {
        self.include_comments = include;
        self
    }

    /// Sets whether to pretty-print the output.
    pub fn with_pretty_print(mut self, pretty: bool) -> Self {
        self.pretty_print = pretty;
        self
    }

    /// Sets the line width scale factor.
    pub fn with_line_width_scale(mut self, scale: f32) -> Self {
        self.line_width_scale = Some(scale);
        self
    }
}
