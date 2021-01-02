#!/bin/bash

if [[ -z $HFS ]]; then
	echo "Must source houdini_setup"
	exit 1
fi

if [[ $(uname) == "Linux" ]]; then
  incl=$HFS/toolkit/include/HAPI
  export LIBCLANG_PATH=/shots/spi/home/software/packages/llvm/11.0.0/gcc-6.3/lib

else
  incl="/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/include/HAPI"
fi

cargo run --release \
--package codegen -- \
--include "${incl}" \
--wrapper codegen/wrapper.h \
--outdir hapi-rs/src/ffi \
--config codegen/codegen.toml \
