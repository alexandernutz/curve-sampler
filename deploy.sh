#!/bin/bash
# 0. Increment version
if [ ! -f .version ]; then echo "0" > .version; fi
OLD_VERSION=$(cat .version)
NEW_VERSION=$((OLD_VERSION + 1))
echo $NEW_VERSION > .version
VERSION="0.1.$NEW_VERSION"

echo "Deploying version: $VERSION"

# 1. Update version in source code and Cargo.toml
sed -i '' "s/const VERSION: .*/const VERSION: \&'static str = \"$VERSION\";/" sampler/src/lib.rs
sed -i '' "s/^version = .*/version = \"$VERSION\"/" sampler/Cargo.toml

# 2. Re-create Info.plist for CLAP and VST3
mkdir -p local_bundle/CurveSampler.clap/Contents/MacOS
mkdir -p local_bundle/CurveSampler.vst3/Contents/MacOS

# CLAP Info.plist
cat > local_bundle/CurveSampler.clap/Contents/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>CurveSampler</string>
    <key>CFBundleIdentifier</key>
    <string>com.alexandernutz.curve-sampler</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleManufacturer</key>
    <string>lx_ntz</string>
    <key>CFBundleName</key>
    <string>Curve Sampler</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# VST3 Info.plist
cat > local_bundle/CurveSampler.vst3/Contents/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>CurveSampler</string>
    <key>CFBundleIdentifier</key>
    <string>com.alexandernutz.curve-sampler.vst3</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleManufacturer</key>
    <string>lx_ntz</string>
    <key>CFBundleName</key>
    <string>Curve Sampler</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# 3. Build Plugin
cargo build --release -p curve-sampler

# 4. Install Locally
cp target/release/libcurve_sampler.dylib local_bundle/CurveSampler.clap/Contents/MacOS/CurveSampler
cp target/release/libcurve_sampler.dylib local_bundle/CurveSampler.vst3/Contents/MacOS/CurveSampler

# Install CLAP
rm -rf ~/Library/Audio/Plug-Ins/CLAP/CurveSampler.clap
cp -R local_bundle/CurveSampler.clap ~/Library/Audio/Plug-Ins/CLAP/
touch ~/Library/Audio/Plug-Ins/CLAP/CurveSampler.clap

# Install VST3
mkdir -p ~/Library/Audio/Plug-Ins/VST3
rm -rf ~/Library/Audio/Plug-Ins/VST3/CurveSampler.vst3
cp -R local_bundle/CurveSampler.vst3 ~/Library/Audio/Plug-Ins/VST3/
touch ~/Library/Audio/Plug-Ins/VST3/CurveSampler.vst3

# 5. Build Web (Curve Jumbler)
echo "Building Curve Jumbler (WASM)..."
cd app
wasm-pack build --target web --release
cd ..
mkdir -p dist
cp index.html dist/
cp -r app/pkg dist/

echo "Done! Local version: $VERSION"
echo "Web build ready in /dist"
