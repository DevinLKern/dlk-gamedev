#!/bin/bash
# compile_shaders.sh
# Compiles all GLSL shaders in the given directory (default = current directory)
# Requires: glslc (from Shaderc / Vulkan SDK)

# Directory to scan (default = current directory)
SHADER_DIR="${1:-.}"

# Output directory for compiled SPIR-V binaries
OUT_DIR="$SHADER_DIR/compiled"
mkdir -p "$OUT_DIR"

echo "Compiling shaders from: $SHADER_DIR"
echo "Output directory: $OUT_DIR"
echo ""

# File extensions to check
EXTENSIONS=("vert" "frag" "comp" "geom" "tesc" "tese")

for ext in "${EXTENSIONS[@]}"; do
    for shader in "$SHADER_DIR"/*."$ext"; do
        # Skip if no files match
        [[ -e "$shader" ]] || continue

        filename=$(basename -- "$shader")
        out_file="$OUT_DIR/${filename}.spv"

        echo "Compiling: $shader -> $out_file"
        glslc "$shader" -o "$out_file"

        if [[ $? -ne 0 ]]; then
            echo "❌ Failed to compile $shader"
        else
            echo "✅ Success"
        fi
        echo ""
    done
done

echo "All done!"

