# espeak_py

Native espeak bindings for Python.

This package provides performance-optimized bindings to the espeak library.
Currently, this only provides text phonemization functions, not invoking full TTS.

On Linux, espeak is statically linked into the built wheel, but there is a runtime dependency on espeak-ng-data.
This can be satisfied by `sudo apt install espeak-ng-data` (or the equivalent for your distro).

On Mac the data and necessary shared library can be installed with `brew install anarchivist/espeak-ng/espeak-ng --without-pcaudiolib --without-waywardgeek-sonic`

## Building

Requires Rust (and cargo) as well as GNU autotools and python packages [`maturin`](https://github.com/PyO3/maturin) and `twine`

1. Clone (including submodules)
2. Build espeak-ng in-tree with:
```
  cd espeak-ng
  ./configure --without-klatt --without-speechplayer --without-mbrola --without-sonic --without-async
  make
  cd ..
```
3. Build wheels with `RUSTFLAGS='-L espeak-ng/src/.libs' maturin build --release` (on Linux this requires the [maturin docker container](https://hub.docker.com/r/konstin2/maturin))
4. Install to local python with `pip install target/wheels/<generated_wheel_name>.whl`
5. Publish to PyPi using `twine`
