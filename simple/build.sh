#!/bin/bash

export CXX= /opt/rh/devtoolset-6/root/bin/g++

pushd _build
cmake ..
make
popd
