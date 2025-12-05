//! SVG converter implementation for WVG documents.
//!
//! This module provides a concrete implementation of the `Converter` trait
//! that outputs SVG (Scalable Vector Graphics) format.

use std::fmt::Write;

use crate::converter::{Converter, ConverterConfig};
use crate::error::WvgResult;
use crate::types::*;
use tracing::{debug, trace};

/// Converter that produces SVG output from WVG documents.
///
/// This converter transforms a parsed WVG document into an SVG string that
/// can be rendered by web browsers and vector graphics applications.
///
/// # Example
///
/// ```ignore
/// use wvg::{BitStream, WvgParser, SvgConverter, Converter};
///
/// let data = std::fs::read("input.wvg")?;
/// let mut bs = BitStream::new(&data);
/// let parser = WvgParser::new(&mut bs);
/// let document = parser.parse()?;
///
/// let converter = SvgConverter::new();
/// let svg = converter.convert(&document)?;
/// std::fs::write("output.svg", svg)?;
/// ```
pub struct SvgConverter {
    /// Configuration options.
    config: ConverterConfig,
}

impl SvgConverter {
    /// Creates a new SVG converter with default configuration.
    pub fn new() -> Self {
        Self {
            config: ConverterConfig::default(),
        }
    }

    /// Creates a new SVG converter with the given configuration.
    pub fn with_config(config: ConverterConfig) -> Self {
        Self { config }
    }
}

impl Default for SvgConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl Converter for SvgConverter {
    type Output = String;

    fn convert(&self, document: &WvgDocument) -> WvgResult<Self::Output> {
        let mut ctx = SvgContext::new(document, &self.config);
        ctx.generate()
    }
}

/// Internal context for SVG generation.
struct SvgContext<'a> {
    /// The source document.
    document: &'a WvgDocument,
    /// Configuration options.
    config: &'a ConverterConfig,
    /// Output buffer.
    output: String,
    /// Indentation level.
    indent: usize,
    /// Group stack for tracking nested groups.
    group_stack: Vec<bool>,
    /// Angle resolution.
    angle_resolution: f64,
    /// Scale resolution.
    scale_resolution: f64,
}

impl<'a> SvgContext<'a> {
    /// Creates a new SVG generation context.
    fn new(document: &'a WvgDocument, config: &'a ConverterConfig) -> Self {
        // Calculate resolutions from generic params
        let gp = &document.header.codec_params.generic_params;
        let angle_resolution = 22.5 / f64::from(1 << gp.angle_resolution);
        let scale_resolution = 0.25 / f64::from(1 << gp.scale_resolution);

        Self {
            document,
            config,
            output: String::with_capacity(4096),
            indent: 0,
            group_stack: Vec::new(),
            angle_resolution,
            scale_resolution,
        }
    }

    /// Generates the complete SVG document.
    fn generate(&mut self) -> WvgResult<String> {
        self.write_header();
        self.write_elements()?;
        self.write_footer();
        Ok(std::mem::take(&mut self.output))
    }

    /// Writes a line with proper indentation.
    fn write_line(&mut self, line: &str) {
        if self.config.pretty_print {
            for _ in 0..self.indent {
                self.output.push_str("  ");
            }
        }
        self.output.push_str(line);
        if self.config.pretty_print {
            self.output.push('\n');
        }
    }

    /// Writes the SVG header.
    fn write_header(&mut self) {
        let (width, height) = match &self.document.header.codec_params.coord_params {
            CoordinateParams::Flat(params) => (params.drawing_width, params.drawing_height),
            CoordinateParams::Compact(_) => (100, 100), // Fallback
        };

        self.write_line("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        self.write_line(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\">",
            width, height
        ));
        self.indent += 1;

        // Write default styles
        self.write_default_styles();
    }

    /// Writes default styles based on the document color configuration.
    fn write_default_styles(&mut self) {
        let cc = &self.document.header.color_config;

        // Background rectangle if color is set
        if let Some(bg) = &cc.background_color {
            let (width, height) = match &self.document.header.codec_params.coord_params {
                CoordinateParams::Flat(params) => (params.drawing_width, params.drawing_height),
                CoordinateParams::Compact(_) => (100, 100),
            };

            self.write_line(&format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                width,
                height,
                color_to_hex(bg)
            ));
        }

        // Define default styles in defs
        self.write_line("<defs>");
        self.indent += 1;

        // Default stroke color
        let stroke = cc
            .default_line_color
            .as_ref()
            .map(color_to_hex)
            .unwrap_or_else(|| "#000000".to_string());

        // Default fill color
        let fill = cc
            .default_fill_color
            .as_ref()
            .map(color_to_hex)
            .unwrap_or_else(|| "none".to_string());

        self.write_line(&format!(
            "<style>path, polyline, line, circle, ellipse, rect {{ stroke: {}; fill: {}; stroke-width: 1; }}</style>",
            stroke, fill
        ));

        self.indent -= 1;
        self.write_line("</defs>");
    }

    /// Writes all elements to the SVG.
    fn write_elements(&mut self) -> WvgResult<()> {
        for element in &self.document.elements {
            self.write_element(element)?;
        }

        // Close any remaining groups
        while !self.group_stack.is_empty() {
            self.group_stack.pop();
            self.indent -= 1;
            self.write_line("</g>");
        }

        Ok(())
    }

    /// Writes a single element.
    fn write_element(&mut self, element: &WvgElement) -> WvgResult<()> {
        trace!("Converting element: {}", element.id);

        match &element.data {
            ElementData::Polyline(pl) => self.write_polyline(element, pl),
            ElementData::CircularPolyline(cp) => self.write_circular_polyline(element, cp),
            ElementData::SimpleShape(ss) => self.write_simple_shape(element, ss),
            ElementData::Reuse(reuse) => self.write_reuse(element, reuse),
            ElementData::GroupStart(gs) => self.write_group_start(element, gs),
            ElementData::GroupEnd => self.write_group_end(),
        }
    }

    /// Writes a polyline element.
    fn write_polyline(&mut self, element: &WvgElement, pl: &PolylineElement) -> WvgResult<()> {
        debug!("Writing polyline {} with {} points", element.id, pl.points.len());

        if pl.points.is_empty() {
            return Ok(());
        }

        let style = self.build_style(&pl.attributes);

        // Single point = draw a small circle (dot)
        if pl.points.len() == 1 {
            let p = &pl.points[0];
            self.write_line(&format!(
                "<circle id=\"{}\" cx=\"{}\" cy=\"{}\" r=\"1.0\" {}/>",
                element.id, p.x, p.y, style
            ));
            return Ok(());
        }

        // Multiple points = path with line segments
        let mut path_data = String::new();
        for (i, point) in pl.points.iter().enumerate() {
            if i == 0 {
                write!(&mut path_data, "M {} {}", point.x, point.y).unwrap();
            } else {
                // Use relative offsets like Python version
                let prev = &pl.points[i - 1];
                let dx = point.x - prev.x;
                let dy = point.y - prev.y;
                write!(&mut path_data, " l {} {}", dx, dy).unwrap();
            }
        }

        self.write_line(&format!(
            "<path id=\"{}\" d=\"{}\" {}/>",
            element.id, path_data, style
        ));

        Ok(())
    }

    /// Writes a circular polyline element (with arc segments).
    fn write_circular_polyline(
        &mut self,
        element: &WvgElement,
        cp: &CircularPolylineElement,
    ) -> WvgResult<()> {
        debug!(
            "Writing circular polyline {} with {} points",
            element.id,
            cp.points.len()
        );

        if cp.points.len() < 2 {
            return Ok(());
        }

        // Convert relative points to absolute and track current position
        let mut path_data = String::new();
        let mut current_x = 0i32;
        let mut current_y = 0i32;

        for (i, pt) in cp.points.iter().enumerate() {
            let (target_x, target_y) = if pt.is_absolute || i < 2 {
                (pt.point.x, pt.point.y)
            } else {
                (current_x + pt.point.x, current_y + pt.point.y)
            };

            if i == 0 {
                // Move to first point
                write!(&mut path_data, "M {} {}", target_x, target_y).unwrap();
            } else {
                let offset_val = pt.curve_offset;

                if offset_val == 0 {
                    // Straight line
                    write!(&mut path_data, " L {} {}", target_x, target_y).unwrap();
                } else {
                    // Arc segment
                    let arc_str = self.compute_arc_command(
                        current_x, current_y,
                        target_x, target_y,
                        offset_val,
                    );
                    write!(&mut path_data, " {}", arc_str).unwrap();
                }
            }

            current_x = target_x;
            current_y = target_y;
        }

        let style = self.build_style(&cp.attributes);
        self.write_line(&format!(
            "<path id=\"{}\" d=\"{}\" {}/>",
            element.id, path_data, style
        ));

        Ok(())
    }

    /// Computes an SVG arc command from two points and a curve offset.
    /// 
    /// Based on the WVG specification for circular polylines, where the curve
    /// offset determines the arc radius and direction.
    fn compute_arc_command(&self, x1: i32, y1: i32, x2: i32, y2: i32, offset: i32) -> String {
        let dx = (x2 - x1) as f64;
        let dy = (y2 - y1) as f64;
        let chord_len = (dx * dx + dy * dy).sqrt();

        if chord_len < 1e-9 {
            return format!("L {} {}", x2, y2);
        }

        // Calculate arc parameters from curve offset
        // n is 4 or 5 bits based on curve_offset_in_bits setting
        let n = if self.document.header.codec_params.generic_params.curve_offset_in_bits.unwrap_or(0) == 1 {
            5
        } else {
            4
        };
        let k = ((1 << n) - 2) as f64;

        let r = offset as f64 / k;
        let e = r * chord_len;

        if e.abs() < 1e-9 {
            return format!("L {} {}", x2, y2);
        }

        // Calculate radius: R = (L²/4 + e²) / (2|e|)
        let radius = (chord_len * chord_len / 4.0 + e * e) / (2.0 * e.abs());

        // Large arc flag: if |r| > 0.5, arc is > 180 degrees
        let large_arc = if r.abs() > 0.5 { 1 } else { 0 };

        // Sweep flag: positive offset means curve bulges to the left of direction
        // In SVG: sweep=1 means clockwise
        let sweep = if offset > 0 { 1 } else { 0 };

        format!(
            "A {:.2} {:.2} 0 {} {} {} {}",
            radius, radius, large_arc, sweep, x2, y2
        )
    }

    /// Writes a simple shape element.
    fn write_simple_shape(
        &mut self,
        element: &WvgElement,
        ss: &SimpleShapeElement,
    ) -> WvgResult<()> {
        debug!("Writing simple shape {}: {:?}", element.id, ss.shape_type);

        let style = self.build_style(&ss.attributes);

        // Since simple shape parsing is incomplete, we just output a placeholder
        match ss.shape_type {
            SimpleShapeType::Rectangle => {
                self.write_line(&format!(
                    "<rect id=\"{}\" x=\"0\" y=\"0\" width=\"10\" height=\"10\" {}/>",
                    element.id, style
                ));
            }
            SimpleShapeType::Ellipse => {
                self.write_line(&format!(
                    "<ellipse id=\"{}\" cx=\"5\" cy=\"5\" rx=\"5\" ry=\"5\" {}/>",
                    element.id, style
                ));
            }
        }

        Ok(())
    }

    /// Writes a reuse element.
    fn write_reuse(&mut self, element: &WvgElement, reuse: &ReuseElement) -> WvgResult<()> {
        debug!(
            "Writing reuse {} referencing element {}",
            element.id, reuse.element_index
        );

        // Find the referenced element
        let ref_id = format!("el_{}", reuse.element_index);
        let transform_str = self.build_transform(&reuse.transform);

        // Handle array parameters
        if let Some(ref array) = reuse.array_params {
            self.write_array_reuse(element, &ref_id, reuse, array, &transform_str)?;
        } else {
            // Single use
            let style = reuse
                .override_attributes
                .as_ref()
                .map(|a| self.build_style(a))
                .unwrap_or_default();

            self.write_line(&format!(
                "<use id=\"{}\" href=\"#{}\" {} {}/>",
                element.id, ref_id, transform_str, style
            ));
        }

        Ok(())
    }

    /// Writes an array of reuse elements.
    fn write_array_reuse(
        &mut self,
        element: &WvgElement,
        ref_id: &str,
        reuse: &ReuseElement,
        array: &ArrayParams,
        base_transform: &str,
    ) -> WvgResult<()> {
        debug!(
            "Writing array reuse: {}x{}",
            array.columns, array.rows
        );

        let width = array.width.unwrap_or(0);
        let height = array.height.unwrap_or(width);
        let style = reuse
            .override_attributes
            .as_ref()
            .map(|a| self.build_style(a))
            .unwrap_or_default();

        let mut instance_idx = 0;
        for row in 0..array.rows {
            for col in 0..array.columns {
                let tx = i32::from(col) * width;
                let ty = i32::from(row) * height;

                let combined_transform = if tx != 0 || ty != 0 {
                    format!("{} translate({}, {})", base_transform, tx, ty)
                } else {
                    base_transform.to_string()
                };

                self.write_line(&format!(
                    "<use id=\"{}_{}_{}\" href=\"#{}\" {} {}/>",
                    element.id, row, col, ref_id, combined_transform.trim(), style
                ));

                instance_idx += 1;
            }
        }

        trace!("Wrote {} array instances", instance_idx);
        Ok(())
    }

    /// Writes a group start element.
    fn write_group_start(
        &mut self,
        element: &WvgElement,
        gs: &GroupStartElement,
    ) -> WvgResult<()> {
        debug!("Writing group start: {}", element.id);

        let transform_str = gs
            .transform
            .as_ref()
            .map(|t| self.build_transform(t))
            .unwrap_or_default();

        let display = if gs.display { "" } else { " display=\"none\"" };

        self.write_line(&format!(
            "<g id=\"{}\" {}{}>",
            element.id, transform_str, display
        ));

        self.indent += 1;
        self.group_stack.push(true);

        Ok(())
    }

    /// Writes a group end element.
    fn write_group_end(&mut self) -> WvgResult<()> {
        debug!("Writing group end");

        if self.group_stack.pop().is_some() {
            self.indent -= 1;
            self.write_line("</g>");
        }

        Ok(())
    }

    /// Builds a transform string from transform data.
    fn build_transform(&self, t: &Transform) -> String {
        let mut parts = Vec::new();

        // Translate
        let tx = t.translate_x.unwrap_or(0);
        let ty = t.translate_y.unwrap_or(0);
        if tx != 0 || ty != 0 {
            parts.push(format!("translate({}, {})", tx, ty));
        }

        // Rotation (around center if specified)
        if let Some(angle_val) = t.angle {
            let degrees = angle_val as f64 * self.angle_resolution;
            let cx = t.cx.unwrap_or(0);
            let cy = t.cy.unwrap_or(0);
            if cx != 0 || cy != 0 {
                parts.push(format!("rotate({} {} {})", degrees, cx, cy));
            } else {
                parts.push(format!("rotate({})", degrees));
            }
        }

        // Scale
        let sx = t.scale_x.map(|v| 1.0 + v as f64 * self.scale_resolution);
        let sy = t.scale_y.map(|v| 1.0 + v as f64 * self.scale_resolution);

        match (sx, sy) {
            (Some(sx_val), Some(sy_val)) => {
                parts.push(format!("scale({} {})", sx_val, sy_val));
            }
            (Some(sx_val), None) => {
                parts.push(format!("scale({})", sx_val));
            }
            _ => {}
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("transform=\"{}\"", parts.join(" "))
        }
    }

    /// Builds a style string from element attributes.
    fn build_style(&self, attrs: &ElementAttributes) -> String {
        let mut styles = Vec::new();

        // Line type (stroke-dasharray)
        if let Some(line_type) = attrs.line_type {
            let dash = match line_type {
                LineType::Solid => None,
                LineType::Dotted => Some("1 3"),
                LineType::Dashed => Some("5 3"),
                LineType::DashDot => Some("5 2 1 2"),
            };
            if let Some(d) = dash {
                styles.push(format!("stroke-dasharray: {}", d));
            }
        }

        // Line width
        if let Some(line_width) = attrs.line_width {
            let scale = self.config.line_width_scale.unwrap_or(1.0);
            let width = match line_width {
                LineWidth::None => 0.0,
                LineWidth::Fine => 1.0 * scale,
                LineWidth::Normal => 2.0 * scale,
                LineWidth::Thick => 3.0 * scale,
            };
            styles.push(format!("stroke-width: {}", width));
        }

        // Line color
        if let Some(ref color) = attrs.line_color {
            styles.push(format!("stroke: {}", color_to_hex(color)));
        }

        // Fill
        if let Some(has_fill) = attrs.fill {
            if has_fill {
                if let Some(ref fill_color) = attrs.fill_color {
                    styles.push(format!("fill: {}", color_to_hex(fill_color)));
                }
                // Otherwise use default fill
            } else {
                styles.push("fill: none".to_string());
            }
        }

        if styles.is_empty() {
            String::new()
        } else {
            format!("style=\"{}\"", styles.join("; "))
        }
    }

    /// Writes the SVG footer.
    fn write_footer(&mut self) {
        self.indent -= 1;
        self.write_line("</svg>");
    }
}

/// Converts a `Color` to a hex string.
fn color_to_hex(color: &Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

