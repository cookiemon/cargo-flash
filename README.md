# cargo-flash

cargo-flash is a minimalistic extension for cargo that builds your crate
and flashes the resulting binary onto your mcu via openocd.

## Usage

cargo-flash expects that an openocd configuration file exists in your
current working directory. Executing cargo-flash will then build your
crate and write your resulting binary to the mcu. (Warning: Will erase
the complete flash.)

Example openocd.cfg (stm32-discovery):
```tcl
# Sample OpenOCD configuration for the STM32F3DISCOVERY development board

# Depending on the hardware revision you got you'll have to pick ONE of these
# interfaces. At any time only one interface should be commented out.

# Revision C (newer revision)
source [find interface/stlink-v2-1.cfg]

# Revision A and B (older revisions)
# source [find interface/stlink-v2.cfg]

source [find target/stm32f3x.cfg]
```

## Current limitations

 - Working directory must be the crate root.
 - Crates that produce multiple binaries are currently not supported.
