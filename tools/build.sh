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
make
