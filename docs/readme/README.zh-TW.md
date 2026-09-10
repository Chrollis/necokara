![Necokara-Banner](../../icons/neco-banner-3502.png)

<div align="center">

<a href="README.zh.md">简体中文</a> ｜
<a href="../../README.md">English</a> ｜
<a href="README.ja.md">日本語</a>

<br>

<div>
<img src="https://img.shields.io/badge/License-Necokara%20License%201.0-4241A5" alt="License: Necokara License 1.0" title="License: Necokara License 1.0" />
<img src="https://img.shields.io/badge/Platform-Windows-blue?logo=windows" alt="Platform: Windows" title="Platform: Windows" />
<img src="https://img.shields.io/badge/Status-under%20development-yellow" alt="Status: under development" title="Status: under development" />
<img src="https://img.shields.io/badge/FFmpeg-%209.0.1-007808?logo=ffmpeg" alt="FFmpeg: 9.0.1" title="FFmpeg: 9.0.1" />
<img src="https://img.shields.io/badge/Python-%203.12.10-3776AB?logo=python" alt="Python: 3.12.10" title="Python: 3.12.10" />
</div>

<br>

<a href="../">文件</a> ｜
<a href="https://github.com/Chrollis/Necokara/issues">Issues</a> ｜
<a href="https://afdian.com/a/chrollis">愛發電</a> ｜
<a href="mailto:chrollis.phrott@outlook.com">電郵</a>

</div>

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

## 星標歷史

> [!TIP]
> 如果本專案對您的生活 / 工作產生了幫助，或者您關注本專案的未來發展，請給專案 Star，這是我們維護這個開源專案的動力 <3

<a href="https://www.star-history.com/?repos=chrollis%2Fnecokara&type=date&legend=top-right">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&theme=dark&legend=top-right" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&legend=top-right" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&legend=top-right" />
 </picture>
</a>

<div align="center"><i>言之不足，故永歌之</i></div>
