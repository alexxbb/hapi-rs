cmake_minimum_required(VERSION 3.13)
project(hapi-cpp-examples)
list(APPEND CMAKE_PREFIX_PATH "$ENV{HFS}\\toolkit\\cmake")
set(CMAKE_VERBOSE_MAKEFILE ON)

find_package(Houdini REQUIRED)

file(GLOB FILES examples/*.cpp)

foreach (file ${FILES})
    get_filename_component(example ${file} NAME_WE)
    add_executable(${example} ${file})
    target_link_libraries(${example} Houdini)
endforeach ()
