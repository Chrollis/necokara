![Necokara-Banner](icons/neco-banner-3502.png)

<div align="center">

<a href="docs/readme/README.zh.md">简体中文</a> ｜
<a href="docs/readme/README.zh-TW.md">繁體中文</a> ｜
<a href="docs/readme/README.ja.md">日本語</a>

<br>

<div>
<img src="https://img.shields.io/badge/license-Necokara%20License%201.0-blue" href="LICENSE"/>
<img src="https://img.shields.io/badge/platform-Windows-blue"/>
<img src="https://img.shields.io/badge/status-under%20development-yellow"/>
</div>

<br>

<a href="docs/">Documentation</a> ｜
<a href="https://github.com/Chrollis/Necokara/issues">Issues</a> ｜
<a href="https://afdian.com/a/chrollis">Afdian</a> ｜
<a href="mailto:chrollis.phrott@outlook.com">Email</a>

</div>

# Necokara

**Status: Under development.**

## Quick Start

### For Users

- Official stable builds (main branch): available in [Releases](https://github.com/Chrollis/necokara/releases).
- Each release includes two installers: `*_cpu-setup.exe` and `*_cuda-setup.exe`.
- Development/preview builds (dev branch): available through [Afdian](https://afdian.com/a/chrollis).

### For Developers

```bash
git clone https://github.com/Chrollis/necokara.git
cd necokara
npm install

# Create minimal runtime: python embed + pip + ffmpeg into binaries/ (+ binaries/mini for packaging)
npm run create-env
# Install AI components (torch CPU by default, models) into binaries/
npm run setup-env
# Users in mainland China can use: npm run setup-env -- --mirror
# Enable hardware acceleration (CUDA): npm run setup-env -- --cuda

# Build NSIS installer
npm run build-nsis
```

## Acknowledgments

See [ACKNOWLEDGMENTS.md](ACKNOWLEDGMENTS.md) for AI assistance and dependency details.

## License

- Project: [Necokara License 1.0](LICENSE)
- End User License Agreement: [EULA (English)](docs/eula/EULA_en.txt)

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting code.
