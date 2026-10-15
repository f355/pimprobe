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

if(VENDOR)
    execute_process(COMMAND "${NM}" -D --defined-only --format=posix "${VENDOR}"
        OUTPUT_VARIABLE exports RESULT_VARIABLE result)
    if(NOT result EQUAL 0)
        message(FATAL_ERROR "Cannot inspect supplied ABI library")
    endif()
    string(REGEX MATCH "_ZTV14AbstractPlugin [A-Za-z] [0-9a-fA-F]+ ([0-9a-fA-F]+)" vtable "${exports}")
    if(NOT vtable)
        message(FATAL_ERROR "Cannot find AbstractPlugin vtable")
    endif()
    set(expected_vtable_size "${CMAKE_MATCH_1}")
endif()
foreach(component PROXY LAUNCHER)
    execute_process(COMMAND "${NM}" -D --undefined-only --format=posix "${${component}}"
        OUTPUT_VARIABLE imports RESULT_VARIABLE result)
    if(NOT result EQUAL 0)
        message(FATAL_ERROR "Cannot inspect ${component}")
    endif()
    string(REPLACE "\n" ";" lines "${imports}")
    set(vendor_count 0)
    foreach(line IN LISTS lines)
        if(line MATCHES "^([^ ]*(AbstractPlugin|SerialThreadManager)[^ ]*) ")
            set(symbol "${CMAKE_MATCH_1}")
            if(VENDOR)
                string(FIND "${exports}" "${symbol} " found)
                if(found EQUAL -1)
                    message(FATAL_ERROR "${component}: vendor does not export ${symbol}")
                endif()
            endif()
            math(EXPR vendor_count "${vendor_count} + 1")
        endif()
    endforeach()
    message(STATUS "${component}: ${vendor_count} vendor ABI imports verified")
    if(VENDOR)
        execute_process(COMMAND "${NM}" -D --defined-only --format=posix "${${component}}"
            OUTPUT_VARIABLE definitions RESULT_VARIABLE result)
        if(NOT result EQUAL 0)
            message(FATAL_ERROR "Cannot inspect ${component} vtable")
        endif()
        string(REGEX MATCH "_ZTVN8pimprobe[0-9]+PimProbe[A-Za-z]+PluginE [A-Za-z] [0-9a-fA-F]+ ([0-9a-fA-F]+)" vtable "${definitions}")
        if(NOT vtable OR NOT CMAKE_MATCH_1 STREQUAL expected_vtable_size)
            message(FATAL_ERROR "${component}: plugin vtable size does not match AbstractPlugin")
        endif()
    endif()
endforeach()
