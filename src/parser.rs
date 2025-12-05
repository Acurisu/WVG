//! WVG parser implementation.
//!
//! This module provides the parser for WVG binary data, converting it into
//! structured data types that can be further processed or converted to other formats.

use crate::bitstream::BitStream;
use crate::error::{UnsupportedFeature, WvgError, WvgResult};
use crate::types::*;
use tracing::{debug, info, trace, warn};

/// Parser for WVG binary data.
///
/// The parser reads from a `BitStream` and produces a `WvgDocument` containing
/// all the parsed header information and elements.
pub struct WvgParser<'a> {
    /// The bit stream to read from.
    bs: &'a mut BitStream<'a>,
    /// Element masks from the header.
    element_masks: Vec<bool>,
    /// Attribute masks from the header.
    attribute_masks: AttributeMasks,
    /// Generic parameters from the header.
    generic_params: GenericParams,
    /// Whether using compact coordinate mode.
    is_compact: bool,
    /// Flat coordinate parameters (if using flat mode).
    flat_params: Option<FlatCoordinateParams>,
    /// Current offset X use flag for elements.
    offset_x_use: bool,
    /// Current offset Y use flag for elements.
    offset_y_use: bool,
    /// Parsed elements.
    elements: Vec<WvgElement>,
    /// Current element index.
    element_index: usize,
}

impl<'a> WvgParser<'a> {
    pub fn new(bs: &'a mut BitStream<'a>) -> Self {
        Self {
            bs,
            element_masks: Vec::new(),
            attribute_masks: AttributeMasks::default(),
            generic_params: GenericParams::default(),
            is_compact: false,
            flat_params: None,
            offset_x_use: false,
            offset_y_use: false,
            elements: Vec::new(),
            element_index: 0,
        }
    }

    /// Parses the WVG data and returns a structured document.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data is malformed
    /// - An unsupported feature is encountered
    /// - End of stream is reached unexpectedly
    pub fn parse(mut self) -> WvgResult<WvgDocument> {
        let wvg_type = self.bs.read_bit()?;

        if wvg_type == 0 {
            info!("Parsing Character Size WVG");
            return Err(WvgError::UnsupportedFeature(UnsupportedFeature::CharacterSizeWvg));
        }

        info!("Parsing Standard WVG");
        let header = self.parse_standard_wvg_header()?;
        self.parse_elements()?;

        Ok(WvgDocument {
            header,
            elements: self.elements,
        })
    }

    fn parse_standard_wvg_header(&mut self) -> WvgResult<WvgHeader> {
        debug!("--- Header ---");

        let general_info = self.parse_general_info()?;
        let color_config = self.parse_color_configuration()?;
        let (codec_params, animation_mode) = self.parse_codec_parameters()?;

        Ok(WvgHeader {
            general_info,
            color_config,
            codec_params,
            animation_mode,
        })
    }

    /// Parses general information from the header.
    fn parse_general_info(&mut self) -> WvgResult<GeneralInfo> {
        let version = self.bs.read_bits(4)? as u8;
        info!("Version: {}", version);

        let mut info = GeneralInfo {
            version,
            ..Default::default()
        };

        let has_extended_info = self.bs.read_bit()?;
        if has_extended_info == 1 {
            let text_code_mode_bit = self.bs.read_bit()?;
            info.text_code_mode = Some(if text_code_mode_bit == 1 {
                TextCodeMode::Ucs2
            } else {
                TextCodeMode::Gsm7Bit
            });
            debug!(
                "Text Code Mode: {}",
                if text_code_mode_bit == 1 { "UCS-2" } else { "GSM 7-bit" }
            );

            // Parse author string
            info.author = self.parse_optional_string(info.text_code_mode.unwrap())?;
            // Parse title string
            info.title = self.parse_optional_string(info.text_code_mode.unwrap())?;
            // Parse timestamp
            info.timestamp = self.parse_timestamp()?;
        }

        Ok(info)
    }

    /// Parses an optional string (author or title).
    ///
    /// Note: This is not fully implemented, string decoding is currently skipped
    /// (raw character bits are consumed elsewhere and an empty string returned).
    /// Proper GSM 7-bit and UCS-2 decoding should be implemented here.
    fn parse_optional_string(&mut self, text_code_mode: TextCodeMode) -> WvgResult<Option<String>> {
        let has_string = self.bs.read_bit()?;
        if has_string == 0 {
            return Ok(None);
        }

        let length = self.bs.read_bits(8)? as usize;
        debug!("String length: {}", length);

        let char_bits = match text_code_mode {
            TextCodeMode::Ucs2 => 16,
            TextCodeMode::Gsm7Bit => 7,
        };

        // Skip characters for now (string handling is complex)
        for _ in 0..length {
            self.bs.read_bits(char_bits)?;
        }

        // TODO: Actually decode the string
        Ok(Some(String::new()))
    }

    fn parse_timestamp(&mut self) -> WvgResult<Option<Timestamp>> {
        let has_timestamp = self.bs.read_bit()?;
        if has_timestamp == 0 {
            return Ok(None);
        }

        let year = self.bs.read_signed_bits(13)? as i16;
        let month = self.bs.read_bits(4)? as u8;
        let day = self.bs.read_bits(5)? as u8;
        let hour = self.bs.read_bits(5)? as u8;
        let minute = self.bs.read_bits(6)? as u8;
        let second = self.bs.read_bits(6)? as u8;

        info!(
            "Timestamp: {}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        );

        Ok(Some(Timestamp {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }))
    }

    fn parse_color_configuration(&mut self) -> WvgResult<ColorConfig> {
        let scheme = self.parse_color_scheme()?;
        info!("Color Scheme: {:?}", scheme);

        let mut config = ColorConfig {
            scheme,
            ..Default::default()
        };

        // Parse default colors
        // <default colors> := ( 0 | (1 <default line color>)) ...
        let has_line_color = self.bs.read_bit()?;
        if has_line_color == 1 {
            debug!("Has default line color");
            config.default_line_color = Some(self.parse_draw_color(scheme)?);
        }

        let has_fill_color = self.bs.read_bit()?;
        if has_fill_color == 1 {
            debug!("Has default fill color");
            config.default_fill_color = Some(self.parse_draw_color(scheme)?);
        }

        let has_bg_color = self.bs.read_bit()?;
        if has_bg_color == 1 {
            debug!("Has background color");
            config.background_color = Some(self.parse_draw_color(scheme)?);
        }

        Ok(config)
    }

    fn parse_color_scheme(&mut self) -> WvgResult<ColorScheme> {
        let b1 = self.bs.read_bit()?;
        if b1 == 0 {
            let b2 = self.bs.read_bit()?;
            if b2 == 0 {
                return Ok(ColorScheme::BlackAndWhite);
            }
            // 01...
            let b3 = self.bs.read_bit()?;
            if b3 == 0 {
                return Ok(ColorScheme::Grayscale2Bit);
            }
            return Ok(ColorScheme::Predefined2Bit);
        }

        // 1...
        let b2 = self.bs.read_bit()?;
        if b2 == 0 {
            let b3 = self.bs.read_bit()?;
            if b3 == 0 {
                return Ok(ColorScheme::Rgb6Bit);
            }
            return Ok(ColorScheme::Websafe);
        }

        // 11...
        let b3 = self.bs.read_bit()?;
        let b4 = self.bs.read_bit()?;
        let suffix = (b3 << 1) | b4;

        match suffix {
            0 => {
                // 6-bit RGB with palette
                self.parse_6bit_palette()?;
                Ok(ColorScheme::Rgb6BitPalette)
            }
            1 => {
                // Websafe with palette
                self.parse_8bit_palette()?;
                Ok(ColorScheme::WebsafePalette)
            }
            2 => Ok(ColorScheme::Rgb12Bit),
            3 => Ok(ColorScheme::Rgb24Bit),
            _ => unreachable!(),
        }
    }

    fn parse_6bit_palette(&mut self) -> WvgResult<Vec<Color>> {
        let num_colors = self.bs.read_bits(5)? as usize + 1;
        debug!("6-bit Palette: {} colors", num_colors);

        let mut palette = Vec::with_capacity(num_colors);
        for _ in 0..num_colors {
            let rgb = self.bs.read_bits(6)?;
            let r = (((rgb >> 4) & 0x3) * 85) as u8;
            let g = (((rgb >> 2) & 0x3) * 85) as u8;
            let b = ((rgb & 0x3) * 85) as u8;
            palette.push(Color::new(r, g, b));
        }
        Ok(palette)
    }

    fn parse_8bit_palette(&mut self) -> WvgResult<Vec<Color>> {
        let num_colors = self.bs.read_bits(7)? as usize + 1;
        debug!("8-bit Palette: {} colors", num_colors);

        let mut palette = Vec::with_capacity(num_colors);
        for _ in 0..num_colors {
            let index = self.bs.read_bits(8)? as usize;
            palette.push(websafe_color(index));
        }
        Ok(palette)
    }

    /// Parses a draw color based on the color scheme.
    /// 
    /// Note: This function currently does not handle palette lookups for
    /// `Rgb6BitPalette` and `WebsafePalette` schemes. It returns black as a
    /// placeholder in those cases.    
    fn parse_draw_color(&mut self, scheme: ColorScheme) -> WvgResult<Color> {
        match scheme {
            ColorScheme::BlackAndWhite => {
                let bit = self.bs.read_bit()?;
                Ok(if bit == 1 { Color::BLACK } else { Color::WHITE })
            }
            ColorScheme::Grayscale2Bit => {
                let val = self.bs.read_bits(2)?;
                let gray = (val * 85) as u8;
                Ok(Color::new(gray, gray, gray))
            }
            ColorScheme::Predefined2Bit => {
                let val = self.bs.read_bits(2)?;
                Ok(match val {
                    0 => Color::WHITE,
                    1 => Color::new(255, 0, 0), // Red
                    2 => Color::new(0, 255, 0), // Green
                    3 => Color::new(0, 0, 255), // Blue
                    _ => unreachable!(),
                })
            }
            ColorScheme::Rgb6Bit => {
                let rgb = self.bs.read_bits(6)?;
                let r = (((rgb >> 4) & 0x3) * 85) as u8;
                let g = (((rgb >> 2) & 0x3) * 85) as u8;
                let b = ((rgb & 0x3) * 85) as u8;
                Ok(Color::new(r, g, b))
            }
            ColorScheme::Websafe => {
                let index = self.bs.read_bits(8)? as usize;
                Ok(websafe_color(index))
            }
            ColorScheme::Rgb12Bit => {
                let rgb = self.bs.read_bits(12)?;
                let r = (((rgb >> 8) & 0xF) * 17) as u8;
                let g = (((rgb >> 4) & 0xF) * 17) as u8;
                let b = ((rgb & 0xF) * 17) as u8;
                Ok(Color::new(r, g, b))
            }
            ColorScheme::Rgb24Bit => {
                let r = self.bs.read_bits(8)? as u8;
                let g = self.bs.read_bits(8)? as u8;
                let b = self.bs.read_bits(8)? as u8;
                Ok(Color::new(r, g, b))
            }
            ColorScheme::Rgb6BitPalette | ColorScheme::WebsafePalette => {
                // TODO: Implement palette lookup
                warn!("Palette color lookup not fully implemented");
                Ok(Color::BLACK)
            }
        }
    }

    fn parse_codec_parameters(&mut self) -> WvgResult<(CodecParams, Option<AnimationMode>)> {
        debug!("Parsing Codec Parameters...");

        self.parse_element_mask()?;
        self.parse_attribute_mask()?;
        self.parse_generic_parameters()?;
        let coord_params = self.parse_coordinate_parameters()?;
        let animation_mode = self.parse_animation_settings()?;

        Ok((
            CodecParams {
                element_masks: self.element_masks.clone(),
                attribute_masks: self.attribute_masks.clone(),
                generic_params: self.generic_params.clone(),
                coord_params,
            },
            animation_mode,
        ))
    }

    fn parse_element_mask(&mut self) -> WvgResult<()> {
        let mut masks = Vec::new();

        // Read first 8 masks
        for _ in 0..8 {
            masks.push(self.bs.read_bit()? == 1);
        }

        // Check for extension
        let has_extension = self.bs.read_bit()?;
        if has_extension == 1 {
            // Read 5 more masks
            for _ in 0..5 {
                masks.push(self.bs.read_bit()? == 1);
            }
        }

        debug!("Element Masks: {:?}", masks);
        self.element_masks = masks;
        Ok(())
    }

    fn parse_attribute_mask(&mut self) -> WvgResult<()> {
        self.attribute_masks.line_type = self.bs.read_bit()? == 1;
        self.attribute_masks.line_width = self.bs.read_bit()? == 1;
        self.attribute_masks.line_color = self.bs.read_bit()? == 1;
        self.attribute_masks.fill = self.bs.read_bit()? == 1;

        debug!(
            "Attribute Masks: type={}, width={}, color={}, fill={}",
            self.attribute_masks.line_type,
            self.attribute_masks.line_width,
            self.attribute_masks.line_color,
            self.attribute_masks.fill
        );

        Ok(())
    }

    fn parse_generic_parameters(&mut self) -> WvgResult<()> {
        // Angle parameters
        if self.bs.read_bit()? == 1 {
            self.generic_params.angle_resolution = self.bs.read_bits(2)? as u8;
            self.generic_params.angle_in_bits = self.bs.read_bits(3)? as u8;
            debug!(
                "Generic: Angle Res={}, Bits={}",
                self.generic_params.angle_resolution, self.generic_params.angle_in_bits
            );
        } else {
            debug!("Generic: Angle Default (22.5 deg, 3 bits)");
        }

        // Scale parameters
        if self.bs.read_bit()? == 1 {
            self.generic_params.scale_resolution = self.bs.read_bits(2)? as u8;
            self.generic_params.scale_in_bits = self.bs.read_bits(4)? as u8;
            debug!(
                "Generic: Scale Res={}, Bits={}",
                self.generic_params.scale_resolution, self.generic_params.scale_in_bits
            );
        } else {
            debug!("Generic: Scale Default (1/4, 3 bits)");
        }

        // Index parameters
        if self.bs.read_bit()? == 1 {
            self.generic_params.index_in_bits = self.bs.read_bits(4)? as u8;
            debug!("Generic: Index Bits={}", self.generic_params.index_in_bits);
        } else {
            debug!("Generic: Index Bits Default (2 -> 3 bits)");
        }

        // Curve offset bits (appears when circular polyline or polygon mask is set)
        let has_circular = self.element_masks.get(2).copied().unwrap_or(false);
        let has_polygon = self.element_masks.get(8).copied().unwrap_or(false);

        if has_circular || has_polygon {
            self.generic_params.curve_offset_in_bits = Some(self.bs.read_bit()? as u8);
            debug!(
                "Generic: Curve Offset Bits={}",
                self.generic_params.curve_offset_in_bits.unwrap()
            );
        }

        Ok(())
    }

    fn parse_coordinate_parameters(&mut self) -> WvgResult<CoordinateParams> {
        self.is_compact = self.bs.read_bit()? == 1;

        if self.is_compact {
            info!("Coordinate Mode: Compact");
            return Err(WvgError::UnsupportedFeature(
                UnsupportedFeature::CompactCoordinateMode,
            ));
        }

        info!("Coordinate Mode: Flat");
        let params = self.parse_flat_coordinate_parameters()?;
        self.flat_params = Some(params.clone());
        Ok(CoordinateParams::Flat(params))
    }

    fn parse_flat_coordinate_parameters(&mut self) -> WvgResult<FlatCoordinateParams> {
        let drawing_width = self.bs.read_bits(16)? as u16;
        info!("Drawing Width: {}", drawing_width);

        let drawing_height = if self.bs.read_bit()? == 1 {
            self.bs.read_bits(16)? as u16
        } else {
            drawing_width
        };
        info!("Drawing Height: {}", drawing_height);

        let max_x_in_bits = self.bs.read_bits(4)? as u8;
        let max_y_in_bits = self.bs.read_bits(4)? as u8;
        let xy_all_positive = self.bs.read_bit()? == 1;
        let trans_xy_in_bits = self.bs.read_bits(4)? as u8;
        let num_points_in_bits = self.bs.read_bits(4)? as u8;
        let offset_x_in_bits_level1 = self.bs.read_bits(4)? as u8;
        let offset_y_in_bits_level1 = self.bs.read_bits(4)? as u8;
        let offset_x_in_bits_level2 = self.bs.read_bits(4)? as u8;
        let offset_y_in_bits_level2 = self.bs.read_bits(4)? as u8;

        debug!(
            "Flat Params: MaxX={}, MaxY={}, AllPos={}, TransXY={}",
            max_x_in_bits, max_y_in_bits, xy_all_positive, trans_xy_in_bits
        );
        debug!(
            "Offsets Level 1: X={}, Y={}",
            offset_x_in_bits_level1, offset_y_in_bits_level1
        );
        debug!(
            "Offsets Level 2: X={}, Y={}",
            offset_x_in_bits_level2, offset_y_in_bits_level2
        );

        Ok(FlatCoordinateParams {
            drawing_width,
            drawing_height,
            max_x_in_bits,
            max_y_in_bits,
            xy_all_positive,
            trans_xy_in_bits,
            num_points_in_bits,
            offset_x_in_bits_level1,
            offset_y_in_bits_level1,
            offset_x_in_bits_level2,
            offset_y_in_bits_level2,
        })
    }

    fn parse_animation_settings(&mut self) -> WvgResult<Option<AnimationMode>> {
        let has_animation = self.element_masks.get(7).copied().unwrap_or(false);
        if has_animation {
            let mode = self.bs.read_bit()?;
            let animation_mode = if mode == 0 {
                AnimationMode::Simple
            } else {
                AnimationMode::Standard
            };
            info!("Animation Mode: {:?}", animation_mode);
            return Ok(Some(animation_mode));
        }
        Ok(None)
    }

    fn parse_elements(&mut self) -> WvgResult<()> {
        debug!("--- Elements ---");

        // Parse number of elements
        let num_elements = if self.bs.read_bit()? == 0 {
            self.bs.read_bits(7)? as usize
        } else {
            self.bs.read_bits(15)? as usize
        };

        info!("Number of elements: {}", num_elements);

        for _ in 0..num_elements {
            self.parse_element()?;
        }

        Ok(())
    }

    fn parse_element(&mut self) -> WvgResult<()> {
        // Calculate number of bits needed for element type based on mask count
        let ones_count: usize = self.element_masks.iter().filter(|&&x| x).count();
        let bits = match ones_count {
            0 | 1 => 0,
            2 => 1,
            3 | 4 => 2,
            5..=8 => 3,
            _ => 4,
        };

        let elem_type_idx = if bits > 0 {
            self.bs.read_bits(bits)?
        } else {
            0
        };

        // Map element type index to actual type based on mask order
        let mut current_idx = 0u32;
        let mut actual_type: Option<usize> = None;

        for (i, &mask) in self.element_masks.iter().enumerate() {
            if mask {
                if current_idx == elem_type_idx {
                    actual_type = Some(i);
                    break;
                }
                current_idx += 1;
            }
        }

        let actual_type = actual_type.ok_or_else(|| {
            WvgError::InvalidElementType(elem_type_idx)
        })?;

        trace!("Element Type Index: {}, Actual Type: {}", elem_type_idx, actual_type);

        let element_id = format!("el_{}", self.element_index);
        self.element_index += 1;

        let element_data = match actual_type {
            0 => {
                // Local envelope
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::LocalEnvelope));
            }
            1 => {
                // Polyline
                trace!("Parsing Polyline Element");
                self.parse_polyline_element()?
            }
            2 => {
                // Circular Polyline
                trace!("Parsing Circular Polyline Element");
                self.parse_circular_polyline_element()?
            }
            3 => {
                // Bezier Polyline
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::BezierPolyline));
            }
            4 => {
                // Simple Shape
                trace!("Parsing Simple Shape Element");
                self.parse_simple_shape_element()?
            }
            5 => {
                // Reuse
                trace!("Parsing Reuse Element");
                self.parse_reuse_element()?
            }
            6 => {
                // Group
                trace!("Parsing Group Element");
                self.parse_group_element()?
            }
            7 => {
                // Animation
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::SimpleAnimation));
            }
            8 => {
                // Polygon
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::Polygon));
            }
            9 => {
                // Special Shape
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::SpecialShape));
            }
            10 => {
                // Frame
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::FrameElement));
            }
            11 => {
                // Text
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::TextElement));
            }
            12 => {
                // Extended
                return Err(WvgError::UnsupportedFeature(UnsupportedFeature::ExtendedElement));
            }
            _ => {
                return Err(WvgError::InvalidElementType(actual_type as u32));
            }
        };

        self.elements.push(WvgElement {
            id: element_id,
            data: element_data,
        });

        Ok(())
    }

    fn parse_basic_element_header(&mut self) -> WvgResult<ElementAttributes> {
        if self.is_compact {
            return Err(WvgError::UnsupportedFeature(
                UnsupportedFeature::CompactCoordinateMode,
            ));
        }

        // <Offset Bit Use>
        self.offset_x_use = self.bs.read_bit()? == 1;
        self.offset_y_use = self.bs.read_bit()? == 1;

        // Check if any attribute mask is set
        let has_any_attr = self.attribute_masks.line_type
            || self.attribute_masks.line_width
            || self.attribute_masks.line_color
            || self.attribute_masks.fill;

        let mut attributes = ElementAttributes::default();

        if has_any_attr {
            let has_attributes = self.bs.read_bit()? == 1;
            if has_attributes {
                attributes = self.parse_attributes_set()?;
            }
        }

        Ok(attributes)
    }

    /// Parses element attributes based on the attribute masks.
    /// 
    /// Note: While line type and width are parsed, line color and fill color
    /// are currently set to black as placeholders. Full color parsing should be implemented.
    fn parse_attributes_set(&mut self) -> WvgResult<ElementAttributes> {
        let mut attrs = ElementAttributes::default();

        if self.attribute_masks.line_type {
            attrs.line_type = Some(LineType::from(self.bs.read_bits(2)?));
        }

        if self.attribute_masks.line_width {
            attrs.line_width = Some(LineWidth::from(self.bs.read_bits(2)?));
        }

        if self.attribute_masks.line_color {
            // Only read line color if line width is not zero
            let line_width = attrs.line_width.unwrap_or(LineWidth::Fine);
            if !matches!(line_width, LineWidth::None) && self.bs.read_bit()? == 1 {
                // TODO: Parse actual color
                attrs.line_color = Some(Color::BLACK);
            }
        }

        if self.attribute_masks.fill {
            // 0 for no fill; 1 for with fill
            if self.bs.read_bit()? == 1 {
                attrs.fill = Some(true);
                // 0 for default fill color, 1 for specified color
                if self.bs.read_bit()? == 1 {
                    // TODO: Parse actual color
                    attrs.fill_color = Some(Color::BLACK);
                }
            } else {
                attrs.fill = Some(false);
            }
        }

        Ok(attrs)
    }

    fn parse_polyline_element(&mut self) -> WvgResult<ElementData> {
        let attributes = self.parse_basic_element_header()?;
        let mut points = Vec::new();

        let params = self.flat_params.as_ref().unwrap();
        let num_points = self.bs.read_bits(params.num_points_in_bits)? as usize;
        trace!("Polyline Points: {}", num_points);

        // First point (absolute)
        let first_point = self.parse_point()?;
        points.push(first_point);

        // Subsequent points (relative offsets)
        for _ in 0..num_points {
            let (dx, dy) = self.parse_offset()?;
            let last = points.last().unwrap();
            points.push(Point::new(last.x + dx, last.y + dy));
        }

        Ok(ElementData::Polyline(PolylineElement { attributes, points }))
    }

    fn parse_circular_polyline_element(&mut self) -> WvgResult<ElementData> {
        let attributes = self.parse_basic_element_header()?;
        let mut points = Vec::new();

        let curve_hint = self.bs.read_bit()? == 1;
        trace!("Curve Hint: {}", curve_hint);

        let params = self.flat_params.as_ref().unwrap();
        let num_points = self.bs.read_bits(params.num_points_in_bits)? as usize;
        trace!("Circular Polyline Points: {}", num_points);

        // First point (absolute)
        let first_pt = self.parse_point()?;
        points.push(CircularPoint {
            curve_offset: 0,
            point: first_pt,
            is_absolute: true,
        });

        // Second point (absolute) with curve offset
        let offset = self.parse_curve_offset(curve_hint)?;
        let second_pt = self.parse_point()?;
        points.push(CircularPoint {
            curve_offset: offset,
            point: second_pt,
            is_absolute: true,
        });

        // Subsequent points (relative) with curve offsets
        for _ in 0..num_points {
            let offset = self.parse_curve_offset(curve_hint)?;
            let (dx, dy) = self.parse_offset()?;
            points.push(CircularPoint {
                curve_offset: offset,
                point: Point::new(dx, dy),
                is_absolute: false,
            });
        }

        Ok(ElementData::CircularPolyline(CircularPolylineElement {
            attributes,
            points,
        }))
    }

    fn parse_curve_offset(&mut self, curve_hint: bool) -> WvgResult<i32> {
        let mut has_value = true;

        if curve_hint && self.bs.read_bit()? == 0 {
            has_value = false;
        }

        if !has_value {
            return Ok(0);
        }

        let bits = if self.generic_params.curve_offset_in_bits.unwrap_or(0) == 1 {
            5
        } else {
            4
        };

        let val = self.bs.read_signed_bits(bits)?;
        trace!("Curve Offset: {}", val);
        Ok(val)
    }

    fn parse_point(&mut self) -> WvgResult<Point> {
        let params = self.flat_params.as_ref().unwrap();

        let x = if params.xy_all_positive {
            self.bs.read_bits(params.max_x_in_bits)? as i32
        } else {
            self.bs.read_signed_bits(params.max_x_in_bits)?
        };

        let y = if params.xy_all_positive {
            self.bs.read_bits(params.max_y_in_bits)? as i32
        } else {
            self.bs.read_signed_bits(params.max_y_in_bits)?
        };

        trace!("Point: ({}, {})", x, y);
        Ok(Point::new(x, y))
    }

    fn parse_offset(&mut self) -> WvgResult<(i32, i32)> {
        let params = self.flat_params.as_ref().unwrap();

        let x_bits = if self.offset_x_use {
            params.offset_x_in_bits_level2
        } else {
            params.offset_x_in_bits_level1
        };

        let y_bits = if self.offset_y_use {
            params.offset_y_in_bits_level2
        } else {
            params.offset_y_in_bits_level1
        };

        let dx = self.bs.read_signed_bits(x_bits)?;
        let dy = self.bs.read_signed_bits(y_bits)?;

        trace!("Offset: ({}, {})", dx, dy);
        Ok((dx, dy))
    }

    fn parse_simple_shape_element(&mut self) -> WvgResult<ElementData> {
        let attributes = self.parse_basic_element_header()?;

        let shape_type = if self.bs.read_bit()? == 0 {
            SimpleShapeType::Rectangle
        } else {
            SimpleShapeType::Ellipse
        };

        // TODO: Parse full shape data
        warn!("Simple shape parsing is incomplete");

        Ok(ElementData::SimpleShape(SimpleShapeElement {
            shape_type,
            attributes,
        }))
    }

    fn parse_reuse_element(&mut self) -> WvgResult<ElementData> {
        let idx_bits = self.generic_params.index_in_bits + 1;
        let mut elem_index = self.bs.read_bits(idx_bits)?;

        // Heuristic fix for potential index issues
        if elem_index as usize >= self.elements.len() {
            warn!(
                "Reuse Element Index {} out of bounds (max {}). Masking MSB.",
                elem_index,
                self.elements.len().saturating_sub(1)
            );
            let masked_index = elem_index & ((1 << (idx_bits - 1)) - 1);
            if (masked_index as usize) < self.elements.len() {
                trace!("  -> Corrected to {}", masked_index);
                elem_index = masked_index;
            } else {
                trace!("  -> Masked index {} still out of bounds.", masked_index);
            }
        }

        trace!("Reuse Element Index: {}", elem_index);

        let transform = self.parse_transform()?;

        // Array parameters
        let array_params = if self.bs.read_bit()? == 1 {
            Some(self.parse_array_parameter()?)
        } else {
            None
        };

        // Override attributes
        let override_attributes = if self.bs.read_bit()? == 1 {
            Some(self.parse_override_attribute_set()?)
        } else {
            None
        };

        Ok(ElementData::Reuse(ReuseElement {
            element_index: elem_index,
            transform,
            array_params,
            override_attributes,
        }))
    }

    fn parse_array_parameter(&mut self) -> WvgResult<ArrayParams> {
        let columns = (self.bs.read_bits(4)? + 1) as u8;
        trace!("Array Columns: {}", columns);

        let width = if columns > 1 {
            let w = self.parse_x_value()?;
            trace!("Array Width: {}", w);
            Some(w)
        } else {
            None
        };

        let rows = (self.bs.read_bits(4)? + 1) as u8;
        trace!("Array Rows: {}", rows);

        let height = if rows > 1 {
            // 0 | (1 <Y>)
            if self.bs.read_bit()? == 1 {
                let h = self.parse_y_value()?;
                trace!("Array Height: {}", h);
                Some(h)
            } else {
                trace!("Array Height: Same as Width");
                width
            }
        } else {
            None
        };

        Ok(ArrayParams {
            columns,
            rows,
            width,
            height,
        })
    }

    fn parse_x_value(&mut self) -> WvgResult<i32> {
        let params = self.flat_params.as_ref().unwrap();
        if params.xy_all_positive {
            Ok(self.bs.read_bits(params.max_x_in_bits)? as i32)
        } else {
            self.bs.read_signed_bits(params.max_x_in_bits)
        }
    }

    fn parse_y_value(&mut self) -> WvgResult<i32> {
        let params = self.flat_params.as_ref().unwrap();
        if params.xy_all_positive {
            Ok(self.bs.read_bits(params.max_y_in_bits)? as i32)
        } else {
            self.bs.read_signed_bits(params.max_y_in_bits)
        }
    }

    /// Parses override attributes for a reuse element.
    ///
    /// Per spec: `<OverrideAttributeSet> ::= 0 | (1 <line type>)
    ///                                       0 | (1 <line width>)
    ///                                       0 | (1 <line color>)
    ///                                       0 | (1 <fill>)
    ///                                       0 | (1 <fill color>)`
    ///
    /// Note: While line type and width are parsed, line color and fill color
    /// are currently set to black as placeholders. Full color parsing should be implemented.
    fn parse_override_attribute_set(&mut self) -> WvgResult<ElementAttributes> {
        let mut attrs = ElementAttributes::default();

        // 0 | (1 <line type>)
        if self.bs.read_bit()? == 1 {
            attrs.line_type = Some(LineType::from(self.bs.read_bits(2)?));
        }

        // 0 | (1 <line width>)
        if self.bs.read_bit()? == 1 {
            attrs.line_width = Some(LineWidth::from(self.bs.read_bits(2)?));
        }

        // 0 | (1 <line color>)
        if self.bs.read_bit()? == 1 {
            // TODO: Parse line color based on color scheme
            attrs.line_color = Some(Color::BLACK);
        }

        // 0 | (1 <fill>)
        if self.bs.read_bit()? == 1 {
            attrs.fill = Some(self.bs.read_bit()? == 1);
        }

        // 0 | (1 <fill color>)
        if self.bs.read_bit()? == 1 {
            // TODO: Parse fill color based on color scheme
            attrs.fill_color = Some(Color::BLACK);
        }

        Ok(attrs)
    }

    fn parse_group_element(&mut self) -> WvgResult<ElementData> {
        if self.bs.read_bit()? == 0 {
            // Group start
            trace!("Group Start");
            let transform = if self.bs.read_bit()? == 1 {
                Some(self.parse_transform()?)
            } else {
                None
            };
            let display = self.bs.read_bit()? == 1;

            Ok(ElementData::GroupStart(GroupStartElement { transform, display }))
        } else {
            // Group end
            trace!("Group End");
            Ok(ElementData::GroupEnd)
        }
    }

    fn parse_transform(&mut self) -> WvgResult<Transform> {
        let mut t = Transform::default();

        // TranslateX
        if self.bs.read_bit()? == 1 {
            t.translate_x = Some(self.parse_translate_value()?);
        }

        // TranslateY
        if self.bs.read_bit()? == 1 {
            t.translate_y = Some(self.parse_translate_value()?);
        }

        // Optional: Angle, Scale, Center
        if self.bs.read_bit()? == 1 {
            // Angle
            if self.bs.read_bit()? == 1 {
                t.angle = Some(self.parse_angle_value()?);
            }

            // ScaleX
            if self.bs.read_bit()? == 1 {
                t.scale_x = Some(self.parse_scale_value()?);
            }

            // ScaleY
            if self.bs.read_bit()? == 1 {
                t.scale_y = Some(self.parse_scale_value()?);
            }

            // CX
            if self.bs.read_bit()? == 1 {
                t.cx = Some(self.parse_translate_value()?);
            }

            // CY
            if self.bs.read_bit()? == 1 {
                t.cy = Some(self.parse_translate_value()?);
            }
        }

        Ok(t)
    }

    fn parse_translate_value(&mut self) -> WvgResult<i32> {
        let params = self.flat_params.as_ref().unwrap();
        let val = self.bs.read_signed_bits(params.trans_xy_in_bits)?;
        trace!("Translate: {}", val);
        Ok(val)
    }

    fn parse_angle_value(&mut self) -> WvgResult<i32> {
        let bits = self.generic_params.angle_in_bits + 1;
        let val = self.bs.read_signed_bits(bits)?;
        trace!("Angle: {}", val);
        Ok(val)
    }

    fn parse_scale_value(&mut self) -> WvgResult<i32> {
        let bits = self.generic_params.scale_in_bits + 1;
        let val = self.bs.read_signed_bits(bits)?;
        trace!("Scale: {}", val);
        Ok(val)
    }
}

fn websafe_color(index: usize) -> Color {
    const WEBSAFE_PALETTE: [[u8; 3]; 256] = [
        [255, 255, 255], [255, 204, 255], [255, 153, 255], [255, 102, 255],
        [255, 51, 255], [255, 0, 255], [255, 255, 204], [255, 204, 204],
        [255, 153, 204], [255, 102, 204], [255, 51, 204], [255, 0, 204],
        [255, 255, 153], [255, 204, 153], [255, 153, 153], [255, 102, 153],
        [255, 51, 153], [255, 0, 153], [204, 255, 255], [204, 204, 255],
        [204, 153, 255], [204, 102, 255], [204, 51, 255], [204, 0, 255],
        [204, 255, 204], [204, 204, 204], [204, 153, 204], [204, 102, 204],
        [204, 51, 204], [204, 0, 204], [204, 255, 153], [204, 204, 153],
        [204, 153, 153], [204, 102, 153], [204, 51, 153], [204, 0, 153],
        [153, 255, 255], [153, 204, 255], [153, 153, 255], [153, 102, 255],
        [153, 51, 255], [153, 0, 255], [153, 255, 204], [153, 204, 204],
        [153, 153, 204], [153, 102, 204], [153, 51, 204], [153, 0, 204],
        [153, 255, 153], [153, 204, 153], [153, 153, 153], [153, 102, 153],
        [153, 51, 153], [153, 0, 153], [102, 255, 255], [102, 204, 255],
        [102, 153, 255], [102, 102, 255], [102, 51, 255], [102, 0, 255],
        [102, 255, 204], [102, 204, 204], [102, 153, 204], [102, 102, 204],
        [102, 51, 204], [102, 0, 204], [102, 255, 153], [102, 204, 153],
        [102, 153, 153], [102, 102, 153], [102, 51, 153], [102, 0, 153],
        [51, 255, 255], [51, 204, 255], [51, 153, 255], [51, 102, 255],
        [51, 51, 255], [51, 0, 255], [51, 255, 204], [51, 204, 204],
        [51, 153, 204], [51, 102, 204], [51, 51, 204], [51, 0, 204],
        [51, 255, 153], [51, 204, 153], [51, 153, 153], [51, 102, 153],
        [51, 51, 153], [51, 0, 153], [0, 255, 255], [0, 204, 255],
        [0, 153, 255], [0, 102, 255], [0, 51, 255], [0, 0, 255],
        [0, 255, 204], [0, 204, 204], [0, 153, 204], [0, 102, 204],
        [0, 51, 204], [0, 0, 204], [0, 255, 153], [0, 204, 153],
        [0, 153, 153], [0, 102, 153], [0, 51, 153], [0, 0, 153],
        [255, 255, 102], [255, 204, 102], [255, 153, 102], [255, 102, 102],
        [255, 51, 102], [255, 0, 102], [255, 255, 51], [255, 204, 51],
        [255, 153, 51], [255, 102, 51], [255, 51, 51], [255, 0, 51],
        [255, 255, 0], [255, 204, 0], [255, 153, 0], [255, 102, 0],
        [255, 51, 0], [255, 0, 0], [204, 255, 102], [204, 204, 102],
        [204, 153, 102], [204, 102, 102], [204, 51, 102], [204, 0, 102],
        [204, 255, 51], [204, 204, 51], [204, 153, 51], [204, 102, 51],
        [204, 51, 51], [204, 0, 51], [204, 255, 0], [204, 204, 0],
        [204, 153, 0], [204, 102, 0], [204, 51, 0], [204, 0, 0],
        [153, 255, 102], [153, 204, 102], [153, 153, 102], [153, 102, 102],
        [153, 51, 102], [153, 0, 102], [153, 255, 51], [153, 204, 51],
        [153, 153, 51], [153, 102, 51], [153, 51, 51], [153, 0, 51],
        [153, 255, 0], [153, 204, 0], [153, 153, 0], [153, 102, 0],
        [153, 51, 0], [153, 0, 0], [102, 255, 102], [102, 204, 102],
        [102, 153, 102], [102, 102, 102], [102, 51, 102], [102, 0, 102],
        [102, 255, 51], [102, 204, 51], [102, 153, 51], [102, 102, 51],
        [102, 51, 51], [102, 0, 51], [102, 255, 0], [102, 204, 0],
        [102, 153, 0], [102, 102, 0], [102, 51, 0], [102, 0, 0],
        [51, 255, 102], [51, 204, 102], [51, 153, 102], [51, 102, 102],
        [51, 51, 102], [51, 0, 102], [51, 255, 51], [51, 204, 51],
        [51, 153, 51], [51, 102, 51], [51, 51, 51], [51, 0, 51],
        [51, 255, 0], [51, 204, 0], [51, 153, 0], [51, 102, 0],
        [51, 51, 0], [51, 0, 0], [0, 255, 102], [0, 204, 102],
        [0, 153, 102], [0, 102, 102], [0, 51, 102], [0, 0, 102],
        [0, 255, 51], [0, 204, 51], [0, 153, 51], [0, 102, 51],
        [0, 51, 51], [0, 0, 51], [0, 255, 0], [0, 204, 0],
        [0, 153, 0], [0, 102, 0], [0, 51, 0], [17, 17, 17],
        [34, 34, 34], [68, 68, 68], [85, 85, 85], [119, 119, 119],
        [136, 136, 136], [170, 170, 170], [187, 187, 187], [221, 221, 221],
        [238, 238, 238], [192, 192, 192], [128, 0, 0], [128, 0, 128],
        [0, 128, 0], [0, 128, 128], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
        [0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0],
    ];

    let [r, g, b] = WEBSAFE_PALETTE.get(index).copied().unwrap_or([0, 0, 0]);
    Color::new(r, g, b)
}
