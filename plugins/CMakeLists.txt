# PIMProbe - touch probing for the Nestworks C500.
# Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

cmake_minimum_required(VERSION 3.21)
project(pimprobe_plugins LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
include(CTest)
find_package(Qt6 REQUIRED COMPONENTS Core Network Widgets)

add_library(pimprobe_controller_bridge STATIC src/controller_bridge.cpp)
target_include_directories(pimprobe_controller_bridge PUBLIC src)
target_link_libraries(pimprobe_controller_bridge PUBLIC Qt6::Core Qt6::Network)
add_library(pimprobe_launcher_button STATIC src/launcher_button.cpp)
target_include_directories(pimprobe_launcher_button PUBLIC src)
target_link_libraries(pimprobe_launcher_button PUBLIC Qt6::Widgets)
set_target_properties(pimprobe_controller_bridge pimprobe_launcher_button
    PROPERTIES POSITION_INDEPENDENT_CODE ON)

if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    foreach(component controller_bridge launcher_button)
        add_executable(pimprobe_${component}_test tests/${component}_test.cpp)
        target_link_libraries(pimprobe_${component}_test PRIVATE
            pimprobe_${component} Qt6::Test)
        add_test(NAME pimprobe_${component}_test COMMAND pimprobe_${component}_test)
        set_tests_properties(pimprobe_${component}_test PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
    endforeach()
endif()

set(PIMPROBE_ABSTRACT_PLUGIN "" CACHE FILEPATH "Optional CNC_Lab library for linking and ABI verification")
if(PIMPROBE_ABSTRACT_PLUGIN)
    if(NOT EXISTS "${PIMPROBE_ABSTRACT_PLUGIN}")
        message(FATAL_ERROR "PIMPROBE_ABSTRACT_PLUGIN does not exist")
    endif()
    add_library(cnc_lab_abstract_plugin SHARED IMPORTED)
    set_target_properties(cnc_lab_abstract_plugin PROPERTIES
        IMPORTED_LOCATION "${PIMPROBE_ABSTRACT_PLUGIN}")
endif()
if(CMAKE_SYSTEM_NAME STREQUAL "Linux" OR PIMPROBE_ABSTRACT_PLUGIN)
    foreach(component proxy launcher)
        add_library(pimprobe_${component}_plugin MODULE
            src/pimprobe_${component}_plugin.cpp src/pimprobe_${component}_plugin.h)
        target_link_libraries(pimprobe_${component}_plugin PRIVATE Qt6::Widgets)
        if(PIMPROBE_ABSTRACT_PLUGIN)
            target_link_libraries(pimprobe_${component}_plugin PRIVATE cnc_lab_abstract_plugin)
        endif()
        # CNC_Lab loads libabstractplugin.so into the global symbol scope.
        if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
            target_link_options(pimprobe_${component}_plugin PRIVATE
                "LINKER:--allow-shlib-undefined")
        endif()
        set_target_properties(pimprobe_${component}_plugin PROPERTIES
            SKIP_BUILD_RPATH ON OUTPUT_NAME pimprobe${component}plugin PREFIX lib)
    endforeach()
    target_link_libraries(pimprobe_proxy_plugin PRIVATE pimprobe_controller_bridge)
    target_link_libraries(pimprobe_launcher_plugin PRIVATE pimprobe_launcher_button)
    if(BUILD_TESTING)
        add_executable(pimprobe_plugin_metadata_test tests/plugin_metadata_test.cpp)
        target_link_libraries(pimprobe_plugin_metadata_test PRIVATE Qt6::Core)
        add_test(NAME pimprobe_plugin_metadata_test COMMAND pimprobe_plugin_metadata_test
            $<TARGET_FILE:pimprobe_proxy_plugin> $<TARGET_FILE:pimprobe_launcher_plugin>)
        if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
            add_test(NAME pimprobe_plugin_symbols_test COMMAND ${CMAKE_COMMAND}
                -DNM=${CMAKE_NM} -DVENDOR=${PIMPROBE_ABSTRACT_PLUGIN}
                -DPROXY=$<TARGET_FILE:pimprobe_proxy_plugin>
                -DLAUNCHER=$<TARGET_FILE:pimprobe_launcher_plugin>
                -P ${CMAKE_CURRENT_SOURCE_DIR}/tests/check_plugin_symbols.cmake)
        endif()
    endif()
endif()
