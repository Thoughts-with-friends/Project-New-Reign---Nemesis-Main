cmake_minimum_required (VERSION 3.20)

option(NEMESIS_BUILD_APP "Build the Qt launcher (requires Qt 6)" ON)

if(NEMESIS_BUILD_APP)
    add_subdirectory("Nemesis_App")
endif()

add_subdirectory("Nemesis_Core_Engine")
add_subdirectory("Nemesis_Havok")
