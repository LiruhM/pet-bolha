cmake_minimum_required(VERSION 3.22.1)
project("pet_bolha_jni" LANGUAGES CXX)

# Caminho para o pet-core compilado
set(PET_CORE_ROOT ${CMAKE_SOURCE_DIR}/../../pet-core)

# Arquiteturas suportadas
set(ANDROID_ABIS "arm64-v8a;armeabi-v7a")

# Diretório de saída
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_SOURCE_DIR}/src/main/jniLibs/${ANDROID_ABI})

# Inclui headers do pet-core
include_directories(${PET_CORE_ROOT}/include)

# Link contra a lib pet_core
add_library(pet_core SHARED IMPORTED)
set_target_properties(pet_core PROPERTIES
    IMPORTED_LOCATION ${PET_CORE_ROOT}/target/${ANDROID_ABI}/release/libpet_core.so
)

# Wrapper JNI
add_library(pet_bolha_jni SHARED
    src/main/cpp/pet_jni.cpp
)

target_link_libraries(pet_bolha_jni
    pet_core
    ${log-lib}
    ${android-lib}
)