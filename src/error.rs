//! Error types for the WVG library.
//!
//! This module defines all error types that can occur during WVG parsing
//! and conversion operations.

use std::fmt;
use thiserror::Error;

pub type WvgResult<T> = Result<T, WvgError>;

/// Errors that can occur during WVG parsing and conversion.
#[derive(Error, Debug)]
pub enum WvgError {
    /// Reached end of stream while reading data.
    #[error("unexpected end of stream")]
    EndOfStream,

    /// The WVG type indicator is invalid.
    #[error("invalid WVG type: expected 0 (character size) or 1 (standard)")]
    InvalidWvgType,

    /// The color scheme value is invalid.
    #[error("invalid color scheme: {0}")]
    InvalidColorScheme(String),

    /// An element type is invalid or unknown.
    #[error("invalid element type: {0}")]
    InvalidElementType(u32),

    /// A feature is not yet implemented.
    #[error("feature not supported: {0}")]
    UnsupportedFeature(UnsupportedFeature),

    /// Generic parse error with context.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Conversion error.
    #[error("conversion error: {0}")]
    ConversionError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Element index out of bounds in reuse element.
    #[error("element index {index} out of bounds (max: {max})")]
    ElementIndexOutOfBounds {
        /// The invalid index that was referenced.
        index: u32,
        /// The maximum valid index.
        max: usize,
    },
}

/// Features that are not yet implemented in the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedFeature {
    /// Character Size WVG format.
    CharacterSizeWvg,
    /// Compact coordinate mode.
    CompactCoordinateMode,
    /// Bezier polyline elements.
    BezierPolyline,
    /// Polygon elements.
    Polygon,
    /// Special shape elements (regular polygon, star, grid).
    SpecialShape,
    /// Text elements.
    TextElement,
    /// Simple animation elements.
    SimpleAnimation,
    /// Standard animation elements.
    StandardAnimation,
    /// Extended elements.
    ExtendedElement,
    /// Local envelope elements.
    LocalEnvelope,
    /// Frame elements.
    FrameElement,
    /// Simple shape elements (rectangle, ellipse).
    SimpleShape,
}

impl fmt::Display for UnsupportedFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            UnsupportedFeature::CharacterSizeWvg => "Character Size WVG format",
            UnsupportedFeature::CompactCoordinateMode => "Compact coordinate mode",
            UnsupportedFeature::BezierPolyline => "Bezier polyline elements",
            UnsupportedFeature::Polygon => "Polygon elements",
            UnsupportedFeature::SpecialShape => "Special shape elements (regular polygon, star, grid)",
            UnsupportedFeature::TextElement => "Text elements",
            UnsupportedFeature::SimpleAnimation => "Simple animation elements",
            UnsupportedFeature::StandardAnimation => "Standard animation elements",
            UnsupportedFeature::ExtendedElement => "Extended elements",
            UnsupportedFeature::LocalEnvelope => "Local envelope elements",
            UnsupportedFeature::FrameElement => "Frame elements",
            UnsupportedFeature::SimpleShape => "Simple shape elements (rectangle, ellipse)",
        };
        write!(f, "{}", description)
    }
}
