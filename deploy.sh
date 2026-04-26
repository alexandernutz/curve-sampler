#!/bin/bash
VERSION=$1
if [ -z "$VERSION" ]; then
    VERSION="0.1.$(date +%s)"
fi

echo "Deploying version: $VERSION"

# 1. Update source code version
sed -i '' "s/const VERSION: .*/const VERSION: \&'static str = \"$VERSION\";/" plugin/src/lib.rs

# 2. Re-create Info.plist
mkdir -p local_bundle/CurveExtractor.clap/Contents/MacOS
cat > local_bundle/CurveExtractor.clap/Contents/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>CurveExtractor</string>
    <key>CFBundleIdentifier</key>
    <string>com.alexandernutz.curve-extractor</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Curve Extractor</string>
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
cargo build -p curve-extractor

# 4. Install
cp target/debug/libcurve_extractor.dylib local_bundle/CurveExtractor.clap/Contents/MacOS/CurveExtractor
rm -rf ~/Library/Audio/Plug-Ins/CLAP/CurveExtractor.clap
cp -R local_bundle/CurveExtractor.clap ~/Library/Audio/Plug-Ins/CLAP/
touch ~/Library/Audio/Plug-Ins/CLAP/CurveExtractor.clap

echo "Done! Running version: $VERSION"
