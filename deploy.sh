#!/bin/bash
# 0. Increment version
if [ ! -f .version ]; then echo "0" > .version; fi
OLD_VERSION=$(cat .version)
NEW_VERSION=$((OLD_VERSION + 1))
echo $NEW_VERSION > .version
VERSION="0.1.$NEW_VERSION"

echo "Deploying version: $VERSION"

# 1. Update source code version
sed -i '' "s/const VERSION: .*/const VERSION: \&'static str = \"$VERSION\";/" sampler/src/lib.rs

# 2. Re-create Info.plist
mkdir -p local_bundle/CurveSampler.clap/Contents/MacOS
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

# 3. Build
cargo build -p curve-sampler

# 4. Install
cp target/debug/libcurve_sampler.dylib local_bundle/CurveSampler.clap/Contents/MacOS/CurveSampler
rm -rf ~/Library/Audio/Plug-Ins/CLAP/CurveSampler.clap
cp -R local_bundle/CurveSampler.clap ~/Library/Audio/Plug-Ins/CLAP/
touch ~/Library/Audio/Plug-Ins/CLAP/CurveSampler.clap

echo "Done! Running version: $VERSION"
