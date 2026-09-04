![Necokara-Banner](../../icons/neco-banner-3502.png)

<div align="center">

<a href="../../README.md">English</a> ｜
<a href="README.zh-TW.md">繁體中文</a> ｜
<a href="README.ja.md">日本語</a>

<br>

<div>
<img src="https://img.shields.io/badge/license-Necokara%20License%201.0-blue" href="../../LICENSE"/>
<img src="https://img.shields.io/badge/platform-Windows-blue"/>
<img src="https://img.shields.io/badge/status-under%20development-yellow"/>
</div>

<br>

<a href="../">文档</a> ｜
<a href="https://github.com/Chrollis/Necokara/issues">Issues</a> ｜
<a href="https://afdian.com/a/chrollis">爱发电</a> ｜
<a href="mailto:chrollis.phrott@outlook.com">邮箱</a>

</div>

# Necokara

**状态：正在开发中。**

## 快速开始

### 普通用户

- 正式稳定版（main 分支）：在 [Releases](https://github.com/Chrollis/necokara/releases) 中获取。
- 每个 Release 包含两个安装包：`*_cpu-setup.exe` 和 `*_cuda-setup.exe`。
- 开发预览版（dev 分支）：通过 [爱发电](https://afdian.com/a/chrollis) 提供。

### 开发者

```bash
git clone https://github.com/Chrollis/necokara.git
cd necokara
npm install

# 创建最小运行时（python embed + pip + ffmpeg → binaries/，并产出打包用 binaries/mini）
npm run create-env
# 安装 AI 组件（默认 CPU torch + 模型）
npm run setup-env
# 中国大陆用户可改用：npm run setup-env -- --mirror
# 启用硬件加速（CUDA）：npm run setup-env -- --cuda

# 打包 NSIS 安装程序
npm run build-nsis
```

## 致谢

AI 辅助与依赖详情见 [ACKNOWLEDGMENTS.md](../../ACKNOWLEDGMENTS.md)。

## 许可证

- 项目：[Necokara License 1.0](../../LICENSE)
- 最终用户许可协议：[EULA（简体中文）](../eula/EULA_zh-CN.rtf)
- Python 运行时：`bin/LICENSE.python.txt`
- FFmpeg：`bin/LICENSE.ffmpeg.txt`

## 贡献

提交代码前请阅读 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
