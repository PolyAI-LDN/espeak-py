#!/bin/bash
set -e
SCRIPT_DIR=$(cd $(dirname $0); pwd)
PARENT_DIR=$(dirname $SCRIPT_DIR)

echo $PARENT_DIR
cd $PARENT_DIR/espeak-ng
ls
touch ChangeLog.md
./autogen.sh
./configure --without-klatt --without-speechplayer --without-mbrola --without-sonic --without-async
make
