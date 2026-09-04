![Necokara-Banner](../../icons/neco-banner-3502.png)

<div align="center">

<a href="README.zh.md">简体中文</a> ｜
<a href="../../README.md">English</a> ｜
<a href="README.ja.md">日本語</a>

<br>

<div>
<img src="https://img.shields.io/badge/license-Necokara%20License%201.0-blue" href="../../LICENSE"/>
<img src="https://img.shields.io/badge/platform-Windows-blue"/>
<img src="https://img.shields.io/badge/status-under%20development-yellow"/>
</div>

<br>

<a href="../">文件</a> ｜
<a href="https://github.com/Chrollis/Necokara/issues">Issues</a> ｜
<a href="https://afdian.com/a/chrollis">愛發電</a> ｜
<a href="mailto:chrollis.phrott@outlook.com">電郵</a>

</div>

# Necokara

**狀態：正在開發中。**

## 快速開始

### 一般使用者

- 正式穩定版（main 分支）：在 [Releases](https://github.com/Chrollis/necokara/releases) 中取得。
- 每個 Release 包含兩個安裝程式：`*_cpu-setup.exe` 和 `*_cuda-setup.exe`。
- 開發預覽版（dev 分支）：透過 [愛發電](https://afdian.com/a/chrollis) 提供。

### 開發者

```bash
git clone https://github.com/Chrollis/necokara.git
cd necokara
npm install

# 建立最小執行環境（python embed + pip + ffmpeg → binaries/，並產出打包用 binaries/mini）
npm run create-env
# 安裝 AI 元件（預設 CPU torch + 模型）
npm run setup-env
# 中國大陸使用者可改用：npm run setup-env -- --mirror
# 啟用硬體加速（CUDA）：npm run setup-env -- --cuda

# 打包 NSIS 安裝程式
npm run build-nsis
```

## 致謝

AI 輔助與依賴詳情見 [ACKNOWLEDGMENTS.md](../../ACKNOWLEDGMENTS.md)。

## 授權

- 專案：[Necokara License 1.0](../../LICENSE)
- 最終使用者授權合約：[EULA（繁體中文）](../eula/EULA_zh-TW.rtf)
- Python 執行環境：`bin/LICENSE.python.txt`
- FFmpeg：`bin/LICENSE.ffmpeg.txt`

## 貢獻

提交程式碼前請閱讀 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
