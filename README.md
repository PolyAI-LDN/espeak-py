# espeak_py

Native espeak bindings for Python.

This package provides performance-optimized bindings to the espeak library.
Currently, this only provides text phonemization functions, not invoking full TTS.

On Linux, espeak is statically linked into the built wheel, but there is a runtime dependency on espeak-ng-data.
This can be satisfied by `sudo apt install espeak-ng-data` (or the equivalent for your distro).

On Mac the data and necessary shared library can be installed with `brew tap bplevin36/espeak-ng && brew install bplevin36/espeak-ng/espeak-ng --without-pcaudiolib --without-waywardgeek-sonic`. You will need to have XCode tools and GNU autotools installed (if they aren't already).

On macOS under ARM you will need to set `export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib` for the example to work, due to a failure at import to find the dynamic link as ARM homebrew lib directories aren't in the standard search path.

## Usage Example
```
>>> from espeak_py import list_languages, text_to_phonemes
>>> list_languages()
['af', 'am', 'an', 'ar', 'as', 'az', 'ba', 'bg', 'bn', 'bpy', 'bs', 'ca', 'cmn', 'cs', 'cy', 'da', 'de', 'el', 'en-029', 'en-gb', 'en-gb-scotland', 'en-gb-x-gbclan', 'en-gb-x-gbcwmd', 'en-gb-x-rp', 'en-us', 'eo', 'es', 'es-419', 'et', 'eu', 'fa', 'fa-latn', 'fi', 'fr-be', 'fr-ch', 'fr-fr', 'ga', 'gd', 'gn', 'grc', 'gu', 'hak', 'hi', 'hr', 'ht', 'hu', 'hy', 'hyw', 'ia', 'id', 'is', 'it', 'ja', 'jbo', 'ka', 'kk', 'kl', 'kn', 'ko', 'kok', 'ku', 'ky', 'la', 'lfn', 'lt', 'lv', 'mi', 'mk', 'ml', 'mr', 'ms', 'mt', 'my', 'nb', 'nci', 'ne', 'nl', 'om', 'or', 'pa', 'pap', 'pl', 'pt', 'pt-br', 'py', 'quc', 'ro', 'ru', 'ru-lv', 'sd', 'shn', 'si', 'sk', 'sl', 'sq', 'sr', 'sv', 'sw', 'ta', 'te', 'tn', 'tr', 'tt', 'ur', 'uz', 'vi', 'vi-vn-x-central', 'vi-vn-x-south', 'yue']
>>> text_to_phonemes("The quick brown fox", language="en-gb")
'ðə kwˈɪk bɹˈaʊn fˈɒks'
>>> text_to_phonemes("Dale a tu cuerpo alegria, Macarena", language="es")
'dˈale a tu kwˈeɾpo alˈeɣɾia\nmˌakaɾˈena'
>>> text_to_phonemes("There's been a murder", voice_name="English (Scotland)")
'ðerz bˌiːn ɐ mˈʌɹdɜ'
```

## Building

Requires Rust (and cargo) as well as GNU autotools and python packages [`maturin`](https://github.com/PyO3/maturin) and `twine`
On Mac GNU autotools can be installed with `brew install libtool automake autoconf`

1. Clone (including submodules)
2. Build espeak-ng in-tree with:
```
  cd espeak-ng
  ./autogen.sh  # First-time setup
  ./configure --without-klatt --without-speechplayer --without-mbrola --without-sonic --without-async
  make
  cd ..
```
3. Build wheels with `RUSTFLAGS='-L espeak-ng/src/.libs' maturin build --release` (on Linux this requires the [maturin docker container](https://hub.docker.com/r/konstin2/maturin))
4. Install to local python with `pip install target/wheels/<generated_wheel_name>.whl`
5. Publish to PyPi using `twine`

tools/build.sh will automate 1-3 for you if you have the correct tools configured, it is designed to work in a GitHub Action.