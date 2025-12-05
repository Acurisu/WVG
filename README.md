# WVG - Wireless Vector Graphics Parser

A Rust library for parsing and converting WVG (Wireless Vector Graphics) files.

WVG is a compact binary data format for vector graphics defined in 3GPP TS 23.040 Annex G.

## Why

During the [LakeCTF Quals 25–26](https://ctftime.org/event/2944/), there was a challenge called `Pls respond` that included a PCAP file containing SMS messages, one of which referenced a WVG file. At the time, no publicly available tools existed for parsing WVG files, so I created this library to fill that gap.

## Disclaimer

Because this tool was tested only against the WVG file provided in the challenge, itself generated using a custom tool written by the challenge author, it may be fully wrong. Nevertheless, I hope it serves as a useful starting point for anyone working with WVG files.

## Usage

### As a library

```rust
use wvg::{BitStream, WvgParser, SvgConverter, Converter};

let data = std::fs::read("input.wvg")?;
let mut bitstream = BitStream::new(&data);
let parsed = WvgParser::new(&mut bitstream).parse()?;
let svg = SvgConverter::new(&parsed).convert()?;
```

### As a CLI tool

```bash
# Minimal output (only success/failure)
wvg input.wvg -o output.svg

# Log header information
wvg input.wvg -o output.svg -v normal

# Log everything (header + elements)
wvg input.wvg -o output.svg -v verbose
```

## Unsupported Features

Some WVG features are not yet implemented:

- Character Size WVG
- Compact coordinate mode
- Bezier polylines
- Polygons
- Special shapes
- Text elements
- Animation elements
- Extended elements
- Local envelope elements
- Frame elements

Attempting to parse files containing these elements will result in an error.

## License

MIT
