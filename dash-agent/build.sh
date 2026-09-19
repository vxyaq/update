#!/bin/bash
# Buduje dash-agent.jar (moduly .java Dash) — sam javac + ASM, bez gradle/maven.
set -e
cd "$(dirname "$0")"

ASM_VER="9.8"
ASM_URL="https://repo1.maven.org/maven2/org/ow2/asm/asm/${ASM_VER}/asm-${ASM_VER}.jar"

mkdir -p lib build assets/dash/panorama

if [ ! -f lib/asm.jar ]; then
  echo "[dash-agent] pobieranie ASM..."
  curl -sSL -o lib/asm.jar "$ASM_URL"
fi

# logo launchera -> tekstura menu
if [ -f ../public/icon.webp ]; then
  python3 -c "from PIL import Image; Image.open('../public/icon.webp').convert('RGBA').save('assets/dash/logo.png')"
fi

echo "[dash-agent] kompilacja stubs (sygnatury Mojmap, tylko do javac)..."
rm -rf build/stubs build/classes
mkdir -p build/stubs build/classes
find stubs -name '*.java' > build/stubs.list
javac --release 8 -d build/stubs @build/stubs.list

echo "[dash-agent] kompilacja agenta..."
find src -name '*.java' > build/src.list
javac --release 8 -cp "build/stubs:lib/asm.jar" -d build/classes @build/src.list

echo "[dash-agent] pakowanie fat-jar..."
rm -rf build/jar
mkdir -p build/jar
cp -r build/classes/. build/jar/
unzip -qo lib/asm.jar 'org/*' -d build/jar
cp -r assets build/jar/assets
cp NOTICE build/jar/NOTICE.txt
mkdir -p build/jar/META-INF
printf 'Manifest-Version: 1.0\nPremain-Class: com.dash.agent.DashAgent\nCan-Redefine-Classes: true\nCan-Retransform-Classes: true\n' > build/jar/META-INF/MANIFEST.MF
rm -rf build/jar/META-INF/*.SF build/jar/META-INF/*.RSA build/jar/META-INF/*.DSA
rm -f dash-agent.jar
jar cfm dash-agent.jar build/jar/META-INF/MANIFEST.MF -C build/jar .
echo "[dash-agent] gotowe: dash-agent.jar ($(du -h dash-agent.jar | cut -f1))"
