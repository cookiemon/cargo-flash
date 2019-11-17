use cargo_metadata::Message;
use std::env;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitStatus, Stdio};

// Source: https://www.olimex.com/Products/ARM/JTAG/_resources/Manual_PROGRAMMER.pdf
// Removed arm7_9 dcc_downloads enable because it won't work on stm32
// and seems like it's only an optimization kind of thing.
const PROG_DEVICE_PROC: &str = "
proc program_device {binary} {
    # halt the processor
    halt

    # write file to flash memory
    poll
    flash probe 0
    flash write_image erase unlock $binary

    # start execution of the program
    reset run
    sleep 10

    # exit ocd
    shutdown
}
";

enum Error {
    NoImage,
    MultipleImages,
    SubprocessFailed(ExitStatus),
    Io(io::Error),
    SerdeJson(serde_json::error::Error),
}
use Error::*;

impl From<io::Error> for Error {
    fn from(f: io::Error) -> Error {
        Error::Io(f)
    }
}

impl From<serde_json::Error> for Error {
    fn from(f: serde_json::Error) -> Error {
        Error::SerdeJson(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NoImage => write!(f, "No binary image found."),
            MultipleImages => write!(
                f,
                "Found multiple binary images in crate. This is currently not supported."
            ),
            SubprocessFailed(code) => write!(
                f,
                "Subprocess exited with code {}",
                code.code().unwrap_or(-1)
            ),
            Io(err) => write!(f, "io error: {}", err),
            SerdeJson(err) => write!(f, "serialization error: {}", err),
        }
    }
}

// Run cargo build and return the path of the binary image if there
// is *exactly* one.
//
// Will also print compiler messages from cargo build to stderr, albeit
// currently without color highlighting.
fn cargo_build(args: impl Iterator<Item = String>) -> Result<PathBuf, Error> {
    // Filter flash if run as cargo subcommand.
    let args = args.filter(|x| x != "flash");

    // Pass arguments through to cargo build and capture output.
    let mut cargo_build = Command::new("cargo")
        .arg("build")
        .arg("--message-format=json")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    // Print diagnostic messages of cargo and capture path to binary, if any.
    let mut binary = None;
    let output = cargo_build.stdout.take().unwrap();
    for message in cargo_metadata::parse_messages(output) {
        let message = message?;
        match message {
            // Print rendered string and fallback to non-rendered.
            Message::CompilerMessage(msg) => {
                let msg = msg.message.rendered.unwrap_or(msg.message.message);
                eprintln!("{}", msg);
            }
            // Store binary image path for later usage.
            Message::CompilerArtifact(msg) => {
                if msg.executable.is_some() && binary.is_some() {
                    // Multiple binary images found.
                    return Err(MultipleImages);
                } else if msg.executable.is_some() {
                    // Capture binary image.
                    binary = msg.executable;
                }
            }
            _ => (),
        }
    }

    let exit_code = cargo_build.wait()?;
    if !exit_code.success() {
        return Err(SubprocessFailed(exit_code));
    }

    // Enforce that a binary was found.
    if let Some(binary) = binary {
        Ok(binary)
    } else {
        Err(NoImage)
    }
}

// Execute openocd to flash the binary image onto the mcu.
fn openocd_flash(binary: &Path) -> Result<(), Error> {
    let mut openocd_flash = Command::new("openocd")
        .arg("-fopenocd.cfg")
        // Config that should be inside
        .arg("-cinit")
        .arg("-c")
        .arg(PROG_DEVICE_PROC)
        .arg("-c")
        .arg(format!("program_device \"{}\"", binary.to_str().unwrap()))
        .spawn()?;

    let exit_code = openocd_flash.wait()?;
    if !exit_code.success() {
        Err(SubprocessFailed(exit_code))
    } else {
        Ok(())
    }
}

fn run() -> Result<(), Error> {
    // env::args has our program name as first entry, this must be skipped.
    // Otherwise just pass arguments through to cargo build.
    let binary = cargo_build(env::args().skip(1))?;

    // Flash to device.
    openocd_flash(&binary)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {}", error);
        process::exit(2);
    };
}
