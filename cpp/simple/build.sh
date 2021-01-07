#!/bin/bash

#export CXX='/opt/rh/devtoolset-6/root/bin/g++'
export CMAKE_PREFIX_PATH=/net/apps/rhel7/houdini/hfs18.0.530/toolkit/cmake
export CMAKE_PREFIX_PATH=/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Resources/toolkit/cmake

if [[ ! -d _build ]]; then
  mkdir _build
fi

pushd _build
cmake ..
make
popd
