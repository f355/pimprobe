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
    if(vendor_count LESS 10)
        message(FATAL_ERROR "${component}: missing expected vendor ABI imports")
    endif()
    if(component STREQUAL "LAUNCHER" AND imports MATCHES "SerialThreadManager|QLocalServer|QLocalSocket")
        message(FATAL_ERROR "Launcher depends on controller transport")
    endif()
    if(component STREQUAL "PROXY")
        if(NOT imports MATCHES "SerialThreadManager" OR imports MATCHES "QProcess|QPushButton|QApplication")
            message(FATAL_ERROR "Proxy dependencies are not isolated")
        endif()
    endif()
    message(STATUS "${component}: ${vendor_count} vendor ABI imports verified")
endforeach()
