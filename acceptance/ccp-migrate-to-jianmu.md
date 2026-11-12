# 验收：CCP 过渡版迁移到 Jianmu

对应规格：spec/ccp-migrate-to-jianmu.md

## 通过 / 失败标准

1. 通过：过渡安装包运行后，Jianmu 安装到 %LOCALAPPDATA%\\Programs\\jianmu\\jianmu.exe，且文件 sha256 与 latest 中一致。
2. 通过：Jianmu 安装成功后，CCP 的 Uninstall 注册项、桌面与开始菜单快捷方式、程序文件均被移除。
3. 通过：CCP 用户数据与配置目录保持原样。
4. 失败判定：sha256 缺失或不一致时，Jianmu 不落盘，CCP 仍可正常启动。
5. 失败判定：网络不可用时，CCP 保持不变，并显示失败提示与 Jianmu 下载页。
6. 通过：重复运行过渡安装包不报错（Jianmu 已存在、CCP 已卸载均可处理）。
7. 通过：安装包文件名仍为 claude-codex-pro-{版本}-windows-x64-setup.exe，旧 CCP 的更新检查能识别并下载。

## 验证方式与证据

- 在干净的 Windows 虚拟机或沙箱中安装旧版 CCP，再运行过渡安装包，记录注册表与文件系统前后对比。
- 构造错误 sha256 的 latest 响应，验证第 4 条。
- 断网运行，验证第 5 条。
- 运行 scripts/release/verify-release-workflow.js 与 scripts/release/test-windows-nsis-installation.ps1。
- 在旧版 CCP 上点击更新，截图或日志证明走通整条链路。

## 非目标

- 不验证 macOS。
- 不验证 Jianmu 本身的功能。

## 完成所需证据

- 上述每条的命令输出或截图。
- 未能验证的项必须写明原因与剩余风险。
