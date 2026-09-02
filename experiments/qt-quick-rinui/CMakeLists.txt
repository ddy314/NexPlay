cmake_minimum_required(VERSION 3.24)

project(NexPlayQtPrototype VERSION 0.1 LANGUAGES CXX)

if(POLICY CMP0169)
  cmake_policy(SET CMP0169 OLD)
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.6 REQUIRED COMPONENTS Core Gui Qml Quick)

set(RINUI_SOURCE_ROOT "" CACHE PATH "Directory containing the upstream RinUI/RinUI QML module")

if(RINUI_SOURCE_ROOT STREQUAL "")
  include(FetchContent)
  FetchContent_Declare(
    rinui
    GIT_REPOSITORY https://github.com/RinLit-233-shiroko/Rin-UI.git
    GIT_TAG 36ad74c888a39c86568b1b8b566103f216c3aff5
    GIT_SHALLOW TRUE
  )
  FetchContent_Populate(rinui)
  set(RINUI_SOURCE_ROOT "${rinui_SOURCE_DIR}")
endif()

if(NOT EXISTS "${RINUI_SOURCE_ROOT}/RinUI/qmldir")
  message(FATAL_ERROR "RINUI_SOURCE_ROOT must point to a Rin-UI checkout containing RinUI/qmldir")
endif()

add_executable(nexplay-qt main.cpp)

qt_add_resources(prototype_resources resources.qrc)
target_sources(nexplay-qt PRIVATE ${prototype_resources})

target_compile_definitions(nexplay-qt PRIVATE
  NEXPLAY_RINUI_SOURCE_ROOT="${RINUI_SOURCE_ROOT}"
)

target_link_libraries(nexplay-qt PRIVATE
  Qt6::Core
  Qt6::Gui
  Qt6::Qml
  Qt6::Quick
)

install(TARGETS nexplay-qt RUNTIME DESTINATION bin)
