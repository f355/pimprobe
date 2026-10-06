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

# Configure this directory, build, then run ctest for the focused UI tests.
cmake_minimum_required(VERSION 3.21)
project(pimprobe_ui_boundary_tests LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Test Qml Quick QuickControls2)
include(CTest)
qt_add_executable(history_transport_test history_transport_test.cpp)
target_link_libraries(history_transport_test PRIVATE Qt6::Test Qt6::Qml Qt6::Quick Qt6::QuickControls2)
target_compile_definitions(history_transport_test PRIVATE PIMPROBE_UI_DIR="${CMAKE_CURRENT_SOURCE_DIR}/../..")
add_test(NAME history_transport COMMAND history_transport_test)
find_program(QMLTESTRUNNER qmltestrunner REQUIRED)
add_test(NAME numeric_editor COMMAND "${QMLTESTRUNNER}" -input "${CMAKE_CURRENT_SOURCE_DIR}/../tst_NumericEditor.qml")
set_tests_properties(history_transport numeric_editor PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_CONTROLS_STYLE=Basic")
