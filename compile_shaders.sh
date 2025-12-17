#!/bin/bash
# compile_shaders.sh
# Compiles all GLSL shaders in the given directory (default = current directory)
# Requires: glslc (from Shaderc / Vulkan SDK)

# Directory to scan (default = current directory)
SHADER_DIR="shaders"

# Output directory for compiled SPIR-V binaries
OUT_DIR="files/compiled-shaders"
mkdir -p "$OUT_DIR"

echo "Compiling shaders from: $SHADER_DIR"
echo "Output directory: $OUT_DIR"
echo ""

# File extensions to check
EXTENSIONS=("vert" "frag")

for ext in "${EXTENSIONS[@]}"; do
    for shader in "$SHADER_DIR"/*."$ext"; do
        # Skip if no files match
        [[ -e "$shader" ]] || continue

        filename=$(basename -- "$shader")
        out_file="$OUT_DIR/${filename}.spv"

        echo "Compiling: $shader -> $out_file"
        glslc "$shader" -o "$out_file"

        if [[ $? -ne 0 ]]; then
            echo "Error: Failed to compile $shader"
        else
            echo "Success"
        fi
    done
done


