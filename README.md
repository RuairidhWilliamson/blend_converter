# Blend Converter
[![](https://img.shields.io/crates/v/blend-converter)](https://crates.io/crates/blend-converter)
[![](https://img.shields.io/crates/l/blend-converter)](https://github.com/RuairidhWilliamson/blend_converter/blob/main/LICENSE.md)
[![](https://img.shields.io/docsrs/blend-converter)](https://docs.rs/blend-converter)

Converts blend files to other 3D formats.

There aren't any export options exposed yet, but I plan to add these when needed or someone asks. https://docs.blender.org/api/current/bpy.ops.export_scene.html

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

## Blender executable
You will need blender installed and either visible in the path or pass a path to ConversionOptions. If you have blender installed using flatpak then this should be detected. For more information about the search strategy see https://docs.rs/blend-converter
