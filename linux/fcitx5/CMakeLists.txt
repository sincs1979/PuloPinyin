cmake_minimum_required(VERSION 3.16)
project(fcitx5-buluo VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT BULUO_ENGINE_LIB)
  set(BULUO_ENGINE_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libbuluo_engine.a")
endif()

find_package(Fcitx5Core REQUIRED)
find_package(Gettext QUIET)

add_library(buluo MODULE src/buluo.cpp)
target_include_directories(buluo PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/../engine-ffi/include"
  ${Fcitx5Core_INCLUDE_DIRS}
)
target_link_libraries(buluo PRIVATE Fcitx5::Core)
if(EXISTS "${BULUO_ENGINE_LIB}")
  target_link_libraries(buluo PRIVATE
    "-Wl,--whole-archive" "${BULUO_ENGINE_LIB}" "-Wl,--no-whole-archive"
    ${CMAKE_DL_LIBS} pthread m
  )
else()
  message(FATAL_ERROR "Rust staticlib missing: ${BULUO_ENGINE_LIB}\nRun: cargo build --release -p engine-ffi")
endif()
set_target_properties(buluo PROPERTIES PREFIX "")

include(GNUInstallDirs)
set(BULUO_FCITX_LIBDIR "${CMAKE_INSTALL_PREFIX}/lib/fcitx5" CACHE PATH "fcitx5 addon dir")
set(BULUO_FCITX_DATADIR "${CMAKE_INSTALL_PREFIX}/share/fcitx5" CACHE PATH "fcitx5 data dir")

install(TARGETS buluo DESTINATION "${BULUO_FCITX_LIBDIR}")
install(FILES addon/buluo.conf DESTINATION "${BULUO_FCITX_DATADIR}/addon")
install(FILES inputmethod/buluo.conf DESTINATION "${BULUO_FCITX_DATADIR}/inputmethod")
