//! Integration tests using the sample WVG data file.
//!
//! These tests verify the parser and SVG converter produce correct output
//! by comparing against known-good results.

use wvg::{BitStream, Converter, SvgConverter, WvgParser};
use wvg::types::*;

/// Sample WVG binary data (data.bin from wvg_parser).
const SAMPLE_DATA: &[u8] = &[
    0x80, 0x0c, 0x80, 0x28, 0x00, 0x40, 0x40, 0x08, 0x1d, 0x6e, 0x66, 0x6a,
    0xa2, 0x40, 0x29, 0xa4, 0x4d, 0x37, 0x05, 0xbd, 0x03, 0x78, 0x83, 0xf5,
    0x30, 0x71, 0xa7, 0x32, 0x49, 0x8a, 0x59, 0x92, 0x57, 0x55, 0x44, 0xa2,
    0x48, 0x78, 0x14, 0x4f, 0x61, 0xcd, 0x4a, 0x91, 0x8a, 0x90, 0x07, 0x40,
    0x1d, 0x30, 0x02, 0x2a, 0xa2, 0x70, 0xb2, 0xe9, 0xf3, 0x84, 0xf0, 0x50,
    0x97, 0x4b, 0x0e, 0x7a, 0x9c, 0xcd, 0xc6, 0x60, 0xeb, 0xae, 0x40, 0xf9,
    0x65, 0x8b, 0x3a, 0xe9, 0x80, 0x04, 0xbb, 0xa0, 0x0c, 0xe9, 0x35, 0x21,
    0x2a, 0xa4, 0x25, 0xd4, 0x02, 0xef, 0xa3, 0xdb, 0xe2, 0x80, 0xa6, 0x35,
    0x18, 0x16, 0xd8, 0x64, 0x40, 0x70, 0xc0,
];

/// Expected SVG output for the sample data.
const EXPECTED_SVG: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8"?><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 32">"#,
    r#"<defs><style>path, polyline, line, circle, ellipse, rect { stroke: "#,
    r#"#000000; fill: none; stroke-width: 1; }</style></defs>"#,
    r#"<circle id="el_0" cx="83" cy="9" r="1.0" />"#,
    r#"<path id="el_1" d="M 83 14 l 0 11" />"#,
    r#"<path id="el_2" d="M 3 15 L 16 15 A 6.58 6.58 0 0 0 3 15 A 8.57 8.57 0 0 0 16 22" />"#,
    r#"<path id="el_3" d="M 18 12 l 10 11" />"#,
    r#"<path id="el_4" d="M 18 23 l 10 -11" />"#,
    r#"<path id="el_5" d="M 34 9 l 0 15" />"#,
    r#"<path id="el_6" d="M 34 15 l 3 0" />"#,
    r#"<path id="el_7" d="M 41 10 A 4.64 4.64 0 0 1 49 10 A 4.06 4.06 0 0 1 49 17 A 4.06 4.06 0 0 1 49 24 A 4.64 4.64 0 0 1 41 24" />"#,
    r#"<path id="el_8" d="M 42 17 l 7 0" />"#,
    r#"<path id="el_9" d="M 58 15 A 5.52 5.52 0 0 1 66 15 L 66 25" />"#,
    r#"<path id="el_10" d="M 58 11 l 0 14" />"#,
    r#"<path id="el_11" d="M 78 12 A 4.23 4.23 0 0 0 70 12 L 77 23 A 3.70 3.70 0 0 1 70 23" />"#,
    r#"<path id="el_12" d="M 89 12 L 89 26 A 4.14 4.14 0 0 0 95 26 L 95 12 A 4.14 4.14 0 0 0 89 12 L 95 26" />"#,
    "<use id=\"el_13\" href=\"#el_9\" transform=\"translate(41, 0)\" />",
    "<use id=\"el_14\" href=\"#el_10\" transform=\"translate(41, 0)\" />",
    "<use id=\"el_15\" href=\"#el_11\" transform=\"translate(40, 0)\" />",
    r#"<path id="el_16" d="M 122 7 A 1.82 1.82 0 0 1 124 10 L 124 15 L 127 18 L 124 21 L 124 26 A 1.82 1.82 0 0 1 122 29" />"#,
    r#"<path id="el_17" d="M 0 28 l 6 0" /></svg>"#,
);

// ============================================================================
// Parser Tests
// ============================================================================

#[test]
fn test_parse_sample_data() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // Verify header
    assert_eq!(doc.header.general_info.version, 0);
    assert_eq!(doc.header.color_config.scheme, ColorScheme::BlackAndWhite);

    // Verify dimensions
    if let CoordinateParams::Flat(params) = &doc.header.codec_params.coord_params {
        assert_eq!(params.drawing_width, 128);
        assert_eq!(params.drawing_height, 32);
    } else {
        panic!("Expected flat coordinate params");
    }

    // Verify element count
    assert_eq!(doc.elements.len(), 18);
}

#[test]
fn test_parse_header_element_masks() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // Element masks: polyline, circular polyline, reuse enabled
    let masks = &doc.header.codec_params.element_masks;
    assert!(!masks[0], "Local envelope should be disabled");
    assert!(masks[1], "Polyline should be enabled");
    assert!(masks[2], "Circular polyline should be enabled");
    assert!(!masks[3], "Bezier should be disabled");
    assert!(!masks[4], "Simple shape should be disabled");
    assert!(masks[5], "Reuse should be enabled");
}

#[test]
fn test_parse_header_attribute_masks() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // All attribute masks should be disabled for this sample
    let attrs = &doc.header.codec_params.attribute_masks;
    assert!(!attrs.line_type);
    assert!(!attrs.line_width);
    assert!(!attrs.line_color);
    assert!(!attrs.fill);
}

#[test]
fn test_parse_flat_coordinate_params() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    if let CoordinateParams::Flat(params) = &doc.header.codec_params.coord_params {
        assert_eq!(params.max_x_in_bits, 7);
        assert_eq!(params.max_y_in_bits, 5);
        assert!(params.xy_all_positive);
        assert_eq!(params.trans_xy_in_bits, 7);
        assert_eq!(params.offset_x_in_bits_level1, 3);
        assert_eq!(params.offset_y_in_bits_level1, 3);
        assert_eq!(params.offset_x_in_bits_level2, 5);
        assert_eq!(params.offset_y_in_bits_level2, 5);
    } else {
        panic!("Expected flat coordinate params");
    }
}

#[test]
fn test_parse_first_element_polyline_single_point() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // First element: single-point polyline at (83, 9)
    let el = &doc.elements[0];
    assert_eq!(el.id, "el_0");

    if let ElementData::Polyline(pl) = &el.data {
        assert_eq!(pl.points.len(), 1);
        assert_eq!(pl.points[0].x, 83);
        assert_eq!(pl.points[0].y, 9);
    } else {
        panic!("Expected polyline element");
    }
}

#[test]
fn test_parse_second_element_polyline_two_points() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // Second element: polyline from (83, 14) to (83, 25)
    let el = &doc.elements[1];
    assert_eq!(el.id, "el_1");

    if let ElementData::Polyline(pl) = &el.data {
        assert_eq!(pl.points.len(), 2);
        assert_eq!(pl.points[0].x, 83);
        assert_eq!(pl.points[0].y, 14);
        assert_eq!(pl.points[1].x, 83);
        assert_eq!(pl.points[1].y, 25);
    } else {
        panic!("Expected polyline element");
    }
}

#[test]
fn test_parse_circular_polyline_element() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // Third element: circular polyline (index 2)
    let el = &doc.elements[2];
    assert_eq!(el.id, "el_2");

    if let ElementData::CircularPolyline(cp) = &el.data {
        // Has 4 points total (2 from num_points + 2 initial)
        assert_eq!(cp.points.len(), 4);
        // First point at (3, 15), no curve offset
        assert_eq!(cp.points[0].point.x, 3);
        assert_eq!(cp.points[0].point.y, 15);
        assert_eq!(cp.points[0].curve_offset, 0);
        // Second point at (16, 15), curve offset is 0 (curve_hint bit was 0)
        assert_eq!(cp.points[1].point.x, 16);
        assert_eq!(cp.points[1].point.y, 15);
        assert_eq!(cp.points[1].curve_offset, 0);
        // Third point: curve_offset = -6, relative offset (-13, 0)
        assert_eq!(cp.points[2].curve_offset, -6);
        // Fourth point: curve_offset = -4
        assert_eq!(cp.points[3].curve_offset, -4);
    } else {
        panic!("Expected circular polyline element");
    }
}

#[test]
fn test_parse_reuse_element() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    // Element 13 is a reuse referencing element 9
    let el = &doc.elements[13];
    assert_eq!(el.id, "el_13");

    if let ElementData::Reuse(reuse) = &el.data {
        assert_eq!(reuse.element_index, 9);
        assert_eq!(reuse.transform.translate_x, Some(41));
        assert_eq!(reuse.transform.translate_y, None);
    } else {
        panic!("Expected reuse element");
    }
}

#[test]
fn test_parse_all_element_types() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    let mut polyline_count = 0;
    let mut circular_count = 0;
    let mut reuse_count = 0;

    for el in &doc.elements {
        match &el.data {
            ElementData::Polyline(_) => polyline_count += 1,
            ElementData::CircularPolyline(_) => circular_count += 1,
            ElementData::Reuse(_) => reuse_count += 1,
            _ => {}
        }
    }

    // Count based on actual parsed elements:
    // el_0, el_1, el_3, el_4, el_5, el_6, el_8, el_10, el_17 = 9 polylines
    // el_2, el_7, el_9, el_11, el_12, el_16 = 6 circular polylines
    // el_13, el_14, el_15 = 3 reuse elements
    assert_eq!(polyline_count, 9, "Expected 9 polyline elements");
    assert_eq!(circular_count, 6, "Expected 6 circular polyline elements");
    assert_eq!(reuse_count, 3, "Expected 3 reuse elements");
    assert_eq!(polyline_count + circular_count + reuse_count, 18);
}

// ============================================================================
// SVG Converter Tests
// ============================================================================

#[test]
fn test_convert_sample_to_svg() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    let converter = SvgConverter::new();
    let svg = converter.convert(&doc).expect("Failed to convert to SVG");

    assert_eq!(svg, EXPECTED_SVG);
}

#[test]
fn test_svg_contains_expected_elements() {
    let mut bs = BitStream::new(SAMPLE_DATA);
    let parser = WvgParser::new(&mut bs);
    let doc = parser.parse().expect("Failed to parse sample data");

    let converter = SvgConverter::new();
    let svg = converter.convert(&doc).expect("Failed to convert to SVG");

    // Check SVG header
    assert!(svg.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(svg.contains(r#"viewBox="0 0 128 32""#));

    // Check for circle (single-point polyline)
    assert!(svg.contains(r#"<circle id="el_0" cx="83" cy="9" r="1.0""#));

    // Check for polyline paths
    assert!(svg.contains(r#"<path id="el_1" d="M 83 14 l 0 11""#));

    // Check for circular polyline with arcs
    assert!(svg.contains(r#"A 6.58 6.58 0 0 0 3 15"#));

    // Check for reuse elements
    assert!(svg.contains("<use id=\"el_13\" href=\"#el_9\" transform=\"translate(41, 0)\""));
}
