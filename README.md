![DMG-2025](/assets/logo_large.png)

# The world's worst Game Boy emulator

My shitty GameBoy emulator written in Rust.
Features save states, input rebinding and custom palettes.

## Installation

Download the latest release and run it.
Alternatively install Cargo, clone the repository and build the executable with `cargo build --release`

## Project goals

-   Fucking run something ✅
-   Play Pokemon ✅
-   Add UI ✅
-   Port to WASM?
-   Game Boy Color emulation?

## Credits

-   [PixelMix font by Andrew Tyler](https://www.dafont.com/pixelmix.font)
-   [Game Boy mockup texture by Indieground](https://resourceboy.com/mockups/top-view-close-up-shot-gameboy-on-floor-mockup/)
-   [All the cool libraries used](/Cargo.toml)

## TO-DO

### App features

-   Fast forwarding

### Audio

-   Sound is popping quite often
-   Filters for nicer sounds?

### Graphics

-   Rendering isn't 100% accurate

### Memory

-   Not all MBC types and features are supported
