#![warn(clippy::unwrap_used, missing_docs)]

//! Blend Converter provides a convenient way to automatically convert blender files (.blend) to
//! other 3D file formats that are easier to work with. Currently the only formats specified are
//! gltf based (see [`OutputFormat`]).
//!
//! # Blender Executable
//! To convert blends we need a blender executable. By default we check the path for `blender` and
//! flatpak but if you need to specify a path use [`ConversionOptions::blender_path`]. For more
//! details about the search strategy see [`BlenderExecutable`].
//!
//! # Example
//!
//! ```
//! use std::path::Path;
//! use blend_converter::ConversionOptions;
//!
//! let input_dir = Path::new("blends");
//! let output_dir = Path::new("gltfs");
//! ConversionOptions::new().convert_dir(input_dir, output_dir).unwrap();
//! ```
//!
//! # Build Script
//! You can use this in your build.rs to automatically convert blender files when they change. To
//! do so your build.rs should look something like:
//!
//! ```no_run
//! use std::path::Path;
//!
//! let input_dir = Path::new("blends");
//! blend_converter::ConversionOptions::default()
//!     .convert_dir_build_script(input_dir)
//!     .expect("failed to convert blends");
//! println!("cargo:rerun-if-changed={}", input_dir.display());
//! println!("cargo:rerun-if-changed=build.rs");
//! ```
//! Then assuming you have `blends/test.blend`, in your code you can open the converted files using something like:
//!
//! ```no_run
//! use std::path::Path;
//!
//! let path = Path::new(env!("OUT_DIR")).join("blends").join("test.glb");
//! let f = std::fs::File::open(path);
//! ```
//!

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use walkdir::WalkDir;

/// ConversionOptions describe how blender files should be converted
#[derive(Debug, Default)]
pub struct ConversionOptions {
    /// Ouput format is the desired file format to convert to
    pub output_format: OutputFormat,
    /// Blender path is an optional override path for where to search for blender. If it is None
    /// [`BlenderExecutable::find`] will be used. Read the documentation there for the search
    /// strategy.
    pub blender_path: Option<PathBuf>,
}

/// The output file format to export to
///
/// The default format is [`OutputFormat::Glb`]
#[derive(Debug, Default)]
pub enum OutputFormat {
    /// glTF Binary (.glb) Exports a single file, with all data packed in binary form
    #[default]
    Glb,
    /// glTF Embedded (.gltf) Exports a single file, with all data packed in JSON
    GltfEmbedded,
    /// glTF Separate (.gltf + .bin + textures) Exports multiple files, with separate JSON, binary
    /// and texture data
    GltfSeparate,
}

impl OutputFormat {
    fn export_script(&self, file_path: &Path) -> String {
        let format = match self {
            Self::Glb => "GLB",
            Self::GltfEmbedded => "GLTF_EMBEDDED",
            Self::GltfSeparate => "GLTF_SEPARATE",
        };
        format!("import bpy; bpy.ops.export_scene.gltf(filepath={file_path:?}, check_existing=False, export_format={format:?})")
    }
}

impl ConversionOptions {
    /// Create a new ConversionOptions with default output format of [`OutputFormat::Glb`].
    ///
    /// This is equivalent to [`ConversionOptions::default`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks a directory and converts all the blend files while preserving the directory
    /// structure.
    pub fn convert_dir(&self, input_dir: &Path, output_dir: &Path) -> Result<(), Error> {
        let blender_exe = BlenderExecutable::find_using_options(self)?;
        for entry in WalkDir::new(input_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if let Ok(m) = entry.metadata() {
                // Ignore directories
                if !m.is_file() {
                    continue;
                }

                let input_path = entry.path();
                let base;
                if let Some(entry_parent) = input_path.parent() {
                    base = entry_parent;
                } else {
                    base = Path::new(".");
                }
                let stem = input_path
                    .file_stem()
                    .ok_or(Error::InvalidInputFile(input_path.to_path_buf()))?;
                let output_path = Path::new(&output_dir).join(base).join(stem);
                std::fs::create_dir_all(output_path.parent().expect("walkdir must have parent"))?;

                self.convert_internal(input_path, &output_path, &blender_exe)?;
            }
        }
        Ok(())
    }

    /// Walks a directory converts all the blend files while preserving the directory structure but
    /// outputs them to OUT_DIR.
    ///
    /// For use in build scripts only.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// let input_dir = Path::new("blends");
    /// blend_converter::ConversionOptions::default()
    ///     .convert_dir_build_script(input_dir)
    ///     .expect("failed to convert blends");
    /// println!("cargo:rerun-if-changed={}", input_dir.display());
    /// println!("cargo:rerun-if-changed=build.rs");
    /// ```
    pub fn convert_dir_build_script(&self, input_dir: &Path) -> Result<(), Error> {
        let output_dir_env =
            env::var("OUT_DIR").expect("OUT_DIR is not set, this must be called from build.rs");
        let output_dir = Path::new(&output_dir_env);
        self.convert_dir(input_dir, output_dir)
    }

    /// Convert an individual blend file
    pub fn convert(&self, input: &Path, output: &Path) -> Result<(), Error> {
        let blender_exe = BlenderExecutable::find_using_options(self)?;
        self.convert_internal(input, output, &blender_exe)
    }

    fn convert_internal(
        &self,
        input: &Path,
        output: &Path,
        blender_exe: &BlenderExecutable,
    ) -> Result<(), Error> {
        let input_file_path = input.canonicalize()?;
        if input_file_path
            .extension()
            .ok_or(Error::InvalidInputFile(input_file_path.clone()))?
            != "blend"
        {
            return Err(Error::InvalidInputFile(input_file_path));
        }

        let status = blender_exe
            .cmd()
            .arg("-b")
            .arg(input_file_path)
            .arg("--python-expr")
            .arg(self.output_format.export_script(output))
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::Export(status))
        }
    }
}

/// The blender executable search strategy
#[derive(Debug, Default)]
pub enum BlenderExecutable {
    /// Invokes blender using `blender` because blender is in the path environment variable
    #[default]
    Normal,
    /// Invokes blender using `flatpak run org.blender.Blender`
    Flatpak,
    /// Invokes blender using path provided by [`ConversionOptions::blender_path`]
    Path(PathBuf),
}

impl BlenderExecutable {
    fn find_using_options(options: &ConversionOptions) -> Result<Self, Error> {
        if let Some(path) = &options.blender_path {
            BlenderExecutable::find_using_path(path)
        } else {
            BlenderExecutable::find()
        }
    }

    /// Find tries [`BlenderExecutable::Normal`] then [`BlenderExecutable::Flatpak`] and returns
    /// the first one that succeeds otherwise returns [`Error::MissingBlenderExecutable`]
    pub fn find() -> Result<Self, Error> {
        vec![Self::Normal, Self::Flatpak]
            .into_iter()
            .find(|x| matches!(x.test(), Ok(true)))
            .ok_or(Error::MissingBlenderExecutable)
    }

    /// Only tries `path` as the blender executable and if it succeeds returns
    /// [`BlenderExecutable::Path`] with the `path` otherwise returns
    /// [`Error::MissingBlenderExecutable`]
    pub fn find_using_path(path: &Path) -> Result<Self, Error> {
        let s = Self::Path(path.to_path_buf());
        if matches!(s.test(), Ok(true)) {
            Ok(s)
        } else {
            Err(Error::MissingBlenderExecutable)
        }
    }

    fn cmd(&self) -> Command {
        match self {
            Self::Normal => Command::new("blender"),
            Self::Flatpak => {
                let mut command = Command::new("flatpak");
                command.arg("run").arg("org.blender.Blender");
                command
            }
            Self::Path(path) => Command::new(path),
        }
    }

    fn test(&self) -> std::io::Result<bool> {
        Ok(self.cmd().arg("-b").arg("-v").status()?.success())
    }
}

/// Errors for converting blends
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Could not locate blender executable, see [`BlenderExecutable`] for the search strategy
    #[error("could not locate blender executable, is blender in your path?")]
    MissingBlenderExecutable,
    /// Invalid input file blend
    #[error("invalid input path {0:?}")]
    InvalidInputFile(PathBuf),
    /// Export failed with exit code
    #[error("export failed with exit code {0}")]
    Export(ExitStatus),
    /// IOError when exporting
    #[error("io error occurred: {0}")]
    IOError(#[from] std::io::Error),
}
