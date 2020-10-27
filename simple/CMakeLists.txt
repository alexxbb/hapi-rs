cmake_minimum_required(VERSION 3.13)
project(hapi)
set(CMAKE_VERBOSE_MAKEFILE ON)


set(SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/simple.cpp)

add_executable(simple ${SOURCES})
target_include_directories(simple PRIVATE /net/apps/rhel7/houdini/hfs18.5.351/toolkit/include)
target_link_libraries(simple /net/apps/rhel7/houdini/hfs18.5.351/dsolib/libHAPI.so)

