# Granular Plugin
A Granular synthesis library. Includes a delay effect, which is available as a plugin.

## Granular
The library centers around a granular manager, which allows for granular playback of an audio buffer, loaded from .wav
The manager can manage multiple grains, which are pitched via a global pitch, and each grain is also repitchable individually.
Grain manager has a global ADSR envelope, smoothing (crossfade) on grains, reversing and looping grains.

## Delay
The delay is a stereo delay, with single tap delays, saturation and filtering (in the feedback loop).
Has controllable feedback, mix and time, and has toggleable saturation and filtering. delay can be synced to a tempo with multiple time divisions.


## Building

After installing [Rust](https://rustup.rs/), you can compile Granular Plugin as follows:

```shell
cargo xtask bundle granular_plugin --release
```
This creates a VST3 plugin file, which is also available for Windows and Linux distribution through GitHub action - automated build.


