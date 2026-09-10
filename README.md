![Necokara-Banner](icons/neco-banner-3502.png)

<div align="center">

<a href="docs/readme/README.zh.md">简体中文</a> ｜
<a href="docs/readme/README.zh-TW.md">繁體中文</a> ｜
<a href="docs/readme/README.ja.md">日本語</a>

<br>

<div>
<img src="https://img.shields.io/badge/License-Necokara%20License%201.0-4241A5" alt="License: Necokara License 1.0" title="License: Necokara License 1.0" />
<img src="https://img.shields.io/badge/Platform-Windows-blue?logo=windows" alt="Platform: Windows" title="Platform: Windows" />
<img src="https://img.shields.io/badge/Status-under%20development-yellow" alt="Status: under development" title="Status: under development" />
<img src="https://img.shields.io/badge/FFmpeg-%209.0.1-007808?logo=ffmpeg" alt="FFmpeg: 9.0.1" title="FFmpeg: 9.0.1" />
<img src="https://img.shields.io/badge/Python-%203.12.10-3776AB?logo=python" alt="Python: 3.12.10" title="Python: 3.12.10" />
</div>

<br>

<a href="docs/">Documentation</a> ｜
<a href="https://github.com/Chrollis/Necokara/issues">Issues</a> ｜
<a href="https://afdian.com/a/chrollis">Afdian</a> ｜
<a href="mailto:chrollis.phrott@outlook.com">Email</a>

</div>

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

## Star History

> [!TIP]
> If this project has helped you in your life or work, or if you're interested in its future development, please give the project a Star. It's the driving force behind maintaining this open-source project <3

<a href="https://www.star-history.com/?repos=chrollis%2Fnecokara&type=date&legend=top-right">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&theme=dark&legend=top-right" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&legend=top-right" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=chrollis/necokara&type=date&legend=top-right" />
 </picture>
</a>

<div align="center"><i>If words alone are inadequate, we speak them out in sighs</i></div>
