#!/bin/bash
set -e
SCRIPT_DIR=$(cd $(dirname $0); pwd)
PARENT_DIR=$(dirname $SCRIPT_DIR)

echo $PARENT_DIR
cd $PARENT_DIR/espeak-ng
if [[ "$(uname)" == "Darwin" ]]; then
    brew install automake libtool autoconf
    rm CHANGELOG.md
    echo "Changelog dummy" > ChangeLog.md
fi
ls
./autogen.sh
./configure --without-klatt --without-pcaudiolib --without-mbrola --without-sonic --without-async

# Fix error on Ubuntu 22.04+
# See https://stackoverflow.com/questions/76060903/gcc-multiple-definition-of-error-on-ubuntu-22-04-after-updating-from-ubuntu-2
export CFLAGS="-fcommon"
make
