#!/bin/bash
set -e

echo "========================================="
echo "  ClawParty Desktop - macOS 构建脚本"
echo "========================================="

APP_NAME="ClawPartyDesktop"
BUILD_DIR="build"
APP_BUNDLE="$BUILD_DIR/$APP_NAME.app"

# 清理旧构建
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# 创建 .app 目录结构
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# 复制 Info.plist
cp ClawPartyDesktop/Info.plist "$APP_BUNDLE/Contents/"

echo ""
echo "正在编译 Swift 应用..."

# 编译 Swift 源文件
swiftc \
    -o "$BUILD_DIR/$APP_NAME" \
    -framework Cocoa \
    -framework Foundation \
    -framework UserNotifications \
    -framework SwiftUI \
    -framework Combine \
    -target arm64-apple-macosx13.0 \
    ClawPartyDesktop/ClawPartyDesktopApp.swift \
    ClawPartyDesktop/StatusBarController.swift \
    ClawPartyDesktop/ProcessManager.swift \
    ClawPartyDesktop/ConfigManager.swift \
    ClawPartyDesktop/LLMConfigView.swift \
    ClawPartyDesktop/ContentView.swift \
    ClawPartyDesktop/MainPanelView.swift

echo ""
echo "编译完成!"
echo ""

# 复制二进制到 .app
mv "$BUILD_DIR/$APP_NAME" "$APP_BUNDLE/Contents/MacOS/"

# 检查签名
if command -v codesign &> /dev/null; then
    echo "正在签名应用..."
    codesign --force --deep --sign - "$APP_BUNDLE" 2>/dev/null || true
fi

# 移除隔离属性，避免 Gatekeeper 阻止
if command -v xattr &> /dev/null; then
    echo "正在移除隔离属性..."
    xattr -cr "$APP_BUNDLE" 2>/dev/null || true
fi

echo "========================================="
echo "  构建成功!"
echo "========================================="
echo ""
echo "应用位置: $(pwd)/$APP_BUNDLE"
echo ""
echo "启动方式:"
echo "  1. 双击打开 $APP_BUNDLE"
echo "  2. 或运行: open $(pwd)/$APP_BUNDLE"
echo ""
echo "注意: 如果首次启动被阻止，请执行:"
echo "  xattr -cr $(pwd)/$APP_BUNDLE"
echo "  或在 系统设置 > 隐私与安全性 中点击'仍要打开'"
echo ""
