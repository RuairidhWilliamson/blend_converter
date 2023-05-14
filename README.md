# Blend Converter
[![](https://img.shields.io/crates/v/blend_converter)](https://crates.io/crates/blend_converter)
[![](https://img.shields.io/crates/l/blend_converter)](https://github.com/RuairidhWilliamson/blend_converter/blob/main/LICENSE.md)
[![](https://img.shields.io/docsrs/blend_converter)](https://docs.rs/blend_converter)

Converts blend files to other 3D formats.

## Build Script
You can use this in your build script to automatically convert blender files when they change. To do so your build script should look something like:

```
use std::path::Path;

let input_dir = Path::new("blends");
blend_converter::ConversionOptions::default()
    .convert_dir_build_script(input_dir)
    .expect("failed to convert blends");
println!("cargo:rerun-if-changed={}", input_dir.display());
```
Then assuming you have blends/test.blend, in your code you can open the converted files using something like:

```
use std::path::Path;

let path = Path::new(env!("OUT_DIR")).join("blends").join("test.glb");
let f = std::fs::File::open(path);
```
