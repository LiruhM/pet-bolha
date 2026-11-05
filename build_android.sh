#!/bin/bash
# Build pet-core for Android targets using NDK toolchain
# Run this on a machine with Android NDK installed

set -e

# Android NDK path
NDK_HOME="C:/Users/mathe/AppData/Local/Android/Sdk/ndk/26.1.10909125"
TOOLCHAIN_DIR="$NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64"
export PATH="$TOOLCHAIN_DIR/bin:$PATH"

# Targets Android
TARGETS=(
    "aarch64-linux-android:arm64-v8a:aarch64-linux-android34-clang.cmd"
    "armv7-linux-androideabi:armeabi-v7a:armv7a-linux-androideabi34-clang.cmd"
)

cd "$(dirname "$0")/pet-core"

# Install Rust targets
for TARGET_INFO in "${TARGETS[@]}"; do
    TARGET=$(echo "$TARGET_INFO" | cut -d: -f1)
    rustup target add "$TARGET" 2>/dev/null || true
done

# Build for each target
for TARGET_INFO in "${TARGETS[@]}"; do
    TARGET=$(echo "$TARGET_INFO" | cut -d: -f1)
    ABI=$(echo "$TARGET_INFO" | cut -d: -f2)
    CC=$(echo "$TARGET_INFO" | cut -d: -f3)

    echo "=== Building for $TARGET ($ABI) ==="

    # Set up environment for cross-compilation
    export CC="$TOOLCHAIN_DIR/bin/$CC"
    export CXX="$TOOLCHAIN_DIR/bin/${CC}++"
    export AR="$TOOLCHAIN_DIR/bin/llvm-ar"
    export RANLIB="$TOOLCHAIN_DIR/bin/llvm-ranlib"

    # Cargo config for this target
    mkdir -p .cargo
    cat > .cargo/config.toml <<EOF
[target.$TARGET]
linker = "$CC"
ar = "$AR"
runner = "echo 'Cannot run on host'"

[build]
target = "$TARGET"
EOF

    # Build
    cargo build --target "$TARGET" --release

    # Copy to jniLibs
    DEST="../../pet-android/src/main/jniLibs/$ABI"
    mkdir -p "$DEST"
    # Workspace target dir is at ../target from pet-core
    cp "../target/$TARGET/release/libpet_core.so" "$DEST/"
    echo "Copiado para $DEST/libpet_core.so"
done

# Cleanup
rm -f .cargo/config.toml

echo "=== Build Android completo ==="