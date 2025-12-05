//! WVG data types and structures.
//!
//! This module defines all the data types used to represent a parsed WVG document,
//! including elements, attributes, transforms, and coordinate parameters.

/// A parsed WVG document containing all header information and elements.
#[derive(Debug, Clone)]
pub struct WvgDocument {
    /// The WVG header containing metadata and codec parameters.
    pub header: WvgHeader,
    /// The list of parsed elements.
    pub elements: Vec<WvgElement>,
}

/// WVG document header containing all header information.
#[derive(Debug, Clone)]
pub struct WvgHeader {
    /// General information about the WVG.
    pub general_info: GeneralInfo,
    /// Color configuration.
    pub color_config: ColorConfig,
    /// Codec parameters for parsing.
    pub codec_params: CodecParams,
    /// Animation settings (if animation elements exist).
    pub animation_mode: Option<AnimationMode>,
}

/// General information from the WVG header.
#[derive(Debug, Clone, Default)]
pub struct GeneralInfo {
    /// WVG format version.
    pub version: u8,
    /// Text encoding mode for strings.
    pub text_code_mode: Option<TextCodeMode>,
    /// Author string (if present).
    pub author: Option<String>,
    /// Title string (if present).
    pub title: Option<String>,
    /// Timestamp (if present).
    pub timestamp: Option<Timestamp>,
}

/// Text encoding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCodeMode {
    /// 7-bit GSM character set.
    Gsm7Bit,
    /// 16-bit UCS-2 encoding.
    Ucs2,
}

/// Timestamp information.
#[derive(Debug, Clone)]
pub struct Timestamp {
    pub year: i16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// Color configuration.
#[derive(Debug, Clone)]
pub struct ColorConfig {
    /// The color scheme used in this document.
    pub scheme: ColorScheme,
    /// Default line color (BLACK if not specified).
    pub default_line_color: Option<Color>,
    /// Default fill color (BLACK if not specified).
    pub default_fill_color: Option<Color>,
    /// Background color (WHITE if not specified).
    pub background_color: Option<Color>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            scheme: ColorScheme::BlackAndWhite,
            default_line_color: None,
            default_fill_color: None,
            background_color: None,
        }
    }
}

/// Available color schemes in WVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Black and white (2 colors).
    BlackAndWhite,
    /// 2-bit grayscale (4 levels).
    Grayscale2Bit,
    /// 2-bit predefined colors (white, red, green, blue).
    Predefined2Bit,
    /// 6-bit RGB color.
    Rgb6Bit,
    /// 8-bit websafe color.
    Websafe,
    /// 6-bit RGB with custom palette.
    Rgb6BitPalette,
    /// 8-bit websafe with custom palette.
    WebsafePalette,
    /// 12-bit RGB color.
    Rgb12Bit,
    /// 24-bit RGB color.
    Rgb24Bit,
}

/// A color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Creates a new color with the given RGB values.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Black color.
    pub const BLACK: Color = Color::new(0, 0, 0);
    /// White color.
    pub const WHITE: Color = Color::new(255, 255, 255);
}

/// Codec parameters for parsing elements.
#[derive(Debug, Clone)]
pub struct CodecParams {
    /// Element mask indicating which element types are present.
    pub element_masks: Vec<bool>,
    /// Attribute masks.
    pub attribute_masks: AttributeMasks,
    /// Generic parameters.
    pub generic_params: GenericParams,
    /// Coordinate parameters.
    pub coord_params: CoordinateParams,
}

/// Attribute masks indicating which attributes are used.
#[derive(Debug, Clone, Default)]
pub struct AttributeMasks {
    /// True if line type attribute is used.
    pub line_type: bool,
    /// True if line width attribute is used.
    pub line_width: bool,
    /// True if line color attribute is used.
    pub line_color: bool,
    /// True if fill attribute is used.
    pub fill: bool,
}

/// Generic parameters for angles, scales, and indices.
#[derive(Debug, Clone)]
pub struct GenericParams {
    /// Angle resolution (determines angle unit).
    pub angle_resolution: u8,
    /// Number of bits for angle values.
    pub angle_in_bits: u8,
    /// Scale resolution (determines scale unit).
    pub scale_resolution: u8,
    /// Number of bits for scale values.
    pub scale_in_bits: u8,
    /// Number of bits for index values.
    pub index_in_bits: u8,
    /// Number of bits for curve offset (4 or 5).
    pub curve_offset_in_bits: Option<u8>,
}

impl Default for GenericParams {
    fn default() -> Self {
        Self {
            angle_resolution: 3,  // 22.5 degrees
            angle_in_bits: 2,     // 3 bits total
            scale_resolution: 0,  // 1/4
            scale_in_bits: 2,     // 3 bits total
            index_in_bits: 2,     // 3 bits total
            curve_offset_in_bits: None,
        }
    }
}

/// Coordinate system parameters.
#[derive(Debug, Clone)]
pub enum CoordinateParams {
    /// Flat coordinate system parameters.
    Flat(FlatCoordinateParams),
    /// Compact coordinate system parameters (not fully implemented).
    Compact(CompactCoordinateParams),
}

/// Flat coordinate system parameters.
#[derive(Debug, Clone)]
pub struct FlatCoordinateParams {
    /// Drawing width in pixels.
    pub drawing_width: u16,
    /// Drawing height in pixels.
    pub drawing_height: u16,
    /// Number of bits for X coordinates.
    pub max_x_in_bits: u8,
    /// Number of bits for Y coordinates.
    pub max_y_in_bits: u8,
    /// Whether all coordinates are positive.
    pub xy_all_positive: bool,
    /// Number of bits for translation values.
    pub trans_xy_in_bits: u8,
    /// Number of bits for point count.
    pub num_points_in_bits: u8,
    /// Number of bits for X offset at level 1.
    pub offset_x_in_bits_level1: u8,
    /// Number of bits for Y offset at level 1.
    pub offset_y_in_bits_level1: u8,
    /// Number of bits for X offset at level 2.
    pub offset_x_in_bits_level2: u8,
    /// Number of bits for Y offset at level 2.
    pub offset_y_in_bits_level2: u8,
}

/// Compact coordinate system parameters (stub for future implementation).
#[derive(Debug, Clone, Default)]
pub struct CompactCoordinateParams {
    // TODO: Implement when compact coordinate mode is supported
}

/// Animation mode setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationMode {
    /// Simple animation mode.
    Simple,
    /// Standard animation mode.
    Standard,
}

/// A WVG element.
#[derive(Debug, Clone)]
pub struct WvgElement {
    /// Unique identifier for this element.
    pub id: String,
    /// The element data.
    pub data: ElementData,
}

/// Element-specific data.
#[derive(Debug, Clone)]
pub enum ElementData {
    /// A polyline element.
    Polyline(PolylineElement),
    /// A circular polyline element.
    CircularPolyline(CircularPolylineElement),
    /// A group start element.
    GroupStart(GroupStartElement),
    /// A group end element.
    GroupEnd,
    /// A reuse element.
    Reuse(ReuseElement),
    /// A simple shape element.
    SimpleShape(SimpleShapeElement),
}

/// A polyline element consisting of connected line segments.
#[derive(Debug, Clone)]
pub struct PolylineElement {
    /// Element attributes.
    pub attributes: ElementAttributes,
    /// List of points forming the polyline.
    pub points: Vec<Point>,
}

/// A circular polyline element with arc segments.
#[derive(Debug, Clone)]
pub struct CircularPolylineElement {
    /// Element attributes.
    pub attributes: ElementAttributes,
    /// List of points with curve offsets.
    pub points: Vec<CircularPoint>,
}

/// A point in a circular polyline.
#[derive(Debug, Clone)]
pub struct CircularPoint {
    /// The curve offset for the arc to this point (0 = straight line).
    pub curve_offset: i32,
    /// The point coordinates (absolute or relative).
    pub point: Point,
    /// Whether this point is in absolute coordinates.
    pub is_absolute: bool,
}

/// A 2D point.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Creates a new point.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Element attributes.
#[derive(Debug, Clone, Default)]
pub struct ElementAttributes {
    /// Line type (solid, dash, dotted).
    pub line_type: Option<LineType>,
    /// Line width.
    pub line_width: Option<LineWidth>,
    /// Line color.
    pub line_color: Option<Color>,
    /// Whether the element is filled.
    pub fill: Option<bool>,
    /// Fill color (if filled).
    pub fill_color: Option<Color>,
}

/// Line type styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    /// Solid line.
    Solid,
    /// Dashed line.
    Dashed,
    /// Dotted line.
    Dotted,
    /// Dash-dot pattern.
    DashDot,
}

impl From<u32> for LineType {
    fn from(value: u32) -> Self {
        match value {
            0 => LineType::Solid,
            1 => LineType::Dashed,
            2 => LineType::Dotted,
            3 => LineType::DashDot,
            _ => LineType::Solid,
        }
    }
}

/// Line width settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineWidth {
    /// No line.
    None,
    /// Fine line (thinnest).
    Fine,
    /// Normal/medium line.
    Normal,
    /// Thick line.
    Thick,
}

impl From<u32> for LineWidth {
    fn from(value: u32) -> Self {
        match value {
            0 => LineWidth::None,
            1 => LineWidth::Fine,
            2 => LineWidth::Normal,
            3 => LineWidth::Thick,
            _ => LineWidth::Fine,
        }
    }
}

/// A group start element.
#[derive(Debug, Clone)]
pub struct GroupStartElement {
    /// Optional transform applied to the group.
    pub transform: Option<Transform>,
    /// Whether the group is displayed.
    pub display: bool,
}

/// A reuse element that references another element.
#[derive(Debug, Clone)]
pub struct ReuseElement {
    /// Index of the element to reuse.
    pub element_index: u32,
    /// Transform to apply.
    pub transform: Transform,
    /// Array parameters for repeated rendering.
    pub array_params: Option<ArrayParams>,
    /// Override attributes.
    pub override_attributes: Option<ElementAttributes>,
}

/// Array parameters for reuse elements.
#[derive(Debug, Clone)]
pub struct ArrayParams {
    /// Number of columns.
    pub columns: u8,
    /// Number of rows.
    pub rows: u8,
    /// Total width of the array.
    pub width: Option<i32>,
    /// Total height of the array.
    pub height: Option<i32>,
}

/// A transform operation.
#[derive(Debug, Clone, Default)]
pub struct Transform {
    /// X translation.
    pub translate_x: Option<i32>,
    /// Y translation.
    pub translate_y: Option<i32>,
    /// Rotation angle.
    pub angle: Option<i32>,
    /// X scale factor.
    pub scale_x: Option<i32>,
    /// Y scale factor.
    pub scale_y: Option<i32>,
    /// Center X for rotation/scale.
    pub cx: Option<i32>,
    /// Center Y for rotation/scale.
    pub cy: Option<i32>,
}

/// A simple shape element (rectangle or ellipse).
#[derive(Debug, Clone)]
pub struct SimpleShapeElement {
    /// The type of shape.
    pub shape_type: SimpleShapeType,
    /// Element attributes.
    pub attributes: ElementAttributes,
}

/// Simple shape types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleShapeType {
    /// Rectangle shape.
    Rectangle,
    /// Ellipse shape.
    Ellipse,
}
