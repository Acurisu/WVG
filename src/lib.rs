//! # WVG - Wireless Vector Graphics Parser
//!
//! A library for parsing and converting WVG (Wireless Vector Graphics) files.
//!
//! WVG is a compact binary data format for vector graphics defined in 3GPP TS 23.040.
//!
//! ## Example
//!
//! ```rust,ignore
//! use wvg::{BitStream, WvgParser, SvgConverter, Converter};
//!
//! let data = std::fs::read("input.wvg")?;
//! let mut bitstream = BitStream::new(&data);
//! let parsed = WvgParser::new(&mut bitstream).parse()?;
//! let svg = SvgConverter::new(&parsed).convert()?;
//! ```

pub mod bitstream;
pub mod converter;
pub mod error;
pub mod parser;
pub mod svg;
pub mod types;

// Re-export main types for convenient access
pub use bitstream::BitStream;
pub use converter::Converter;
pub use error::{WvgError, WvgResult};
pub use parser::WvgParser;
pub use svg::SvgConverter;
pub use types::*;
