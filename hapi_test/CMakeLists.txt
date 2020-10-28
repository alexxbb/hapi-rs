cmake_minimum_required(VERSION 3.13)
project(hapi)
set(CMAKE_VERBOSE_MAKEFILE ON)
find_package(Houdini        REQUIRED)


set(SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/simple.cpp)

add_executable(simple ${SOURCES})
target_link_libraries(simple Houdini Houdini::HoudiniHAPIL)

