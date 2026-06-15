# Codex++

<p align="center">
  <img src="docs/images/claude-codex-pro-plus.png" alt="Codex++ 鍥炬爣" width="160">
</p>

<p align="center">
  涓枃 | <a href="README_EN.md">English</a>
</p>

<p align="center">
  <img alt="Release" src="https://img.shields.io/github/v/release/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Stars" src="https://img.shields.io/github/stars/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="License" src="https://img.shields.io/github/license/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.85%2B-orange">
  <img alt="Tauri" src="https://img.shields.io/badge/tauri-2.x-24C8DB">
</p>

Codex++ 鏄潰鍚?Codex App 鐨勫閮ㄥ寮哄惎鍔ㄥ櫒鍜岀鐞嗗伐鍏枫€傚畠涓嶄慨鏀?Codex App 鍘熷瀹夎鏂囦欢锛岃€屾槸閫氳繃澶栭儴 launcher 鍚姩 Codex锛屽苟浣跨敤 Chromium DevTools Protocol 娉ㄥ叆澧炲己鑴氭湰銆?
## 蹇€熶娇鐢?
浠?[GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases) 涓嬭浇鏈€鏂扮増瀹夎鍖咃細

- Windows锛歚CodexPlusPlus-*-windows-x64-setup.exe`
- macOS Intel锛歚CodexPlusPlus-*-macos-x64.dmg`
- macOS Apple Silicon锛歚CodexPlusPlus-*-macos-arm64.dmg`

瀹夎鍚庝細鏈変袱涓叆鍙ｏ細

- `Codex++`锛氶潤榛樺惎鍔ㄥ叆鍙ｏ紝涓嶆樉绀虹鐞嗙晫闈紝鍙礋璐ｅ惎鍔?Codex 骞舵敞鍏ュ寮哄姛鑳姐€?- `Codex++ 绠＄悊宸ュ叿`锛歍auri 鎺у埗闈㈡澘锛岀敤浜庡惎鍔ㄣ€佹鏌ャ€佷慨澶嶃€佹洿鏂般€侀厤缃腑杞敞鍏ャ€佺鐞嗗寮哄姛鑳藉拰鐢ㄦ埛鑴氭湰銆?
Windows 瀹夎鍖呬細鍒涘缓妗岄潰鍜屽紑濮嬭彍鍗曞揩鎹锋柟寮忋€俶acOS DMG 浼氬畨瑁?`/Applications/Codex++.app` 鍜?`/Applications/Codex++ 绠＄悊宸ュ叿.app`銆?
## 涓昏鍔熻兘

- Rust 鍚庣鍜岄潤榛?launcher锛屽惎鍔ㄦ椂涓嶄緷璧栭澶栬繍琛屾椂銆?- Tauri + React 绠＄悊宸ュ叿锛屾敮鎸佹繁鑹?娴呰壊鍒囨崲銆?- 澶栭儴 CDP 娉ㄥ叆锛屼笉鏀?`app.asar`锛屼笉鍚?Codex 瀹夎鐩綍鍐欏叆 DLL銆?- 涓浆娉ㄥ叆妯″紡锛氭敮鎸佸涓腑杞厤缃紝鍐欏叆 `CodexPlusPlus` provider锛屽苟鍙垏鍥炲畼鏂?ChatGPT 鐧诲綍鎬併€?- 浼犵粺澧炲己妯″紡锛氭彃浠跺叆鍙ｈВ閿併€佺壒娈婃彃浠跺己鍒跺畨瑁呫€佷細璇濆垹闄ゃ€丮arkdown 瀵煎嚭銆侀」鐩Щ鍔ㄣ€乀imeline 绛夈€?- 鐢ㄦ埛鑴氭湰鐙珛绠＄悊锛屽彲鍦ㄥ惎鍔ㄦ椂娉ㄥ叆鑷畾涔夎剼鏈€?- Provider 鍚屾锛氬惎鍔ㄥ墠鍚屾鏈湴浼氳瘽 metadata锛屽垏鎹緵搴斿晢鍚庢棫浼氳瘽浠嶅彲瑙併€?- Zed 鎵撳紑鍏ュ彛锛氳瘑鍒繙绋?SSH 涓婁笅鏂囧悗锛屽彲浠?Codex 鐩存帴鎵撳紑瀵瑰簲鏂囦欢鍒?Zed Remote Development銆?- Upstream worktree 鍒涘缓锛氬彲浠?`upstream/<base-branch>` 鍒涘缓鏂?worktree锛屽垱寤哄墠鑷姩 fetch 杩滅鍒嗘敮锛岄檷浣庝粠闄堟棫鏈湴 HEAD 娲剧敓瀵艰嚧鐨勫啿绐侀闄┿€?- GitHub Release 鑷姩鏇存柊锛岀鐞嗗伐鍏峰拰闈欓粯鍚姩鍣ㄩ兘浼氭娴嬪彲鐢ㄦ洿鏂般€?- Windows 鍗曞疄渚嬨€佹棤榛戞鍚姩銆佺鐞嗗憳鏉冮檺娓呭崟銆佺郴缁熸闈㈣矾寰勮瘑鍒€?- macOS x64/arm64 鍒嗘灦鏋?DMG锛岄潤榛樺叆鍙ｉ殣钘?Dock 鍥炬爣銆?
## 鐥涚偣涓庤В鍐?
API Key 鐧诲綍妯″紡涓嬶紝Codex 鍘熺敓鎻掍欢鍏ュ彛浼氭彁绀洪渶瑕佺櫥褰?ChatGPT锛屽鑷存彃浠跺姛鑳芥棤娉曟甯镐娇鐢細

![API Key 妯″紡涓嬫彃浠跺叆鍙ｄ笉鍙敤](docs/images/pain-plugin-disabled.png)

Codex 鍘熺敓浼氳瘽鍒楄〃鍙湁褰掓。鍏ュ彛锛屾病鏈夌湡姝ｇ殑鍒犻櫎鎸夐挳锛?
![鍘熺敓浼氳瘽鍒楄〃缂哄皯鍒犻櫎鑳藉姏](docs/images/pain-no-delete-button.png)

Codex++ 鍚姩鍚庝細瑙ｉ攣鎻掍欢鍏ュ彛锛屽苟鍦ㄤ細璇濆垪琛ㄦ偓鍋滄椂鏄剧ず鍒犻櫎鎸夐挳锛?
![Codex++ 瑙ｉ攣鎻掍欢鍏ュ彛骞舵坊鍔犲垹闄ゆ寜閽甝(docs/images/solution-plugin-and-delete.png)

椤堕儴鑿滃崟鏍忎細鍑虹幇 `Codex++`锛屽彲浠ユ煡鐪嬪悗绔姸鎬佸苟鎵撳紑璁剧疆闈㈡澘锛?
![Codex++ 鍚庣鐘舵€佹寚绀虹伅](docs/images/backend-status-indicator.png)
![Codex++ 璁剧疆闈㈡澘](docs/images/settings-panel.png)

## 涓浆娉ㄥ叆

涓浆娉ㄥ叆閫傚悎宸茬粡鍦?Codex/ChatGPT 涓畬鎴愬畼鏂硅处鍙风櫥褰曪紝鍚屾椂甯屾湜鎶婃ā鍨嬭姹傝浆鍒拌嚜瀹氫箟鍏煎 API 鐨勫満鏅€?
杩欑娣峰悎妯″紡鐨勮竟鐣屾槸锛?
- 瀹樻柟 ChatGPT/Codex 鐧诲綍鎬佺户缁礋璐?Codex App 鐨勮处鍙疯兘鍔涘拰鎻掍欢鍏ュ彛銆?- 涓浆閰嶇疆鍙帴绠℃ā鍨嬭姹備娇鐢ㄧ殑 Base URL銆並ey 鍜屾ā鍨嬪悕绉般€?- 鍏煎 API 渚涘簲鍟嗕笉闇€瑕佸浐瀹氫负鏌愪竴瀹讹紱鍙涓婃父鍗忚鍜?Codex 閰嶇疆鍖归厤鍗冲彲銆?- 娓呴櫎 API 妯″紡鍚庡簲鑳藉洖鍒板畼鏂圭櫥褰曟€侊紝缁х画浣跨敤瀹樻柟璐﹀彿鍜屾彃浠躲€?
搴旂敤涓浆娉ㄥ叆鍓嶅缓璁厛鍋氫竴娆℃渶灏忔鏌ワ細

1. 鍏堢‘璁?Codex 宸叉娴嬪埌 ChatGPT 鐧诲綍鐘舵€侊紝鎻掍欢鍏ュ彛鍙敤銆?2. 纭鑷畾涔?Base URL 鍙闂紝骞朵笖鏀寔鎵€閫変笂娓稿崗璁紙渚嬪 Responses 鍏煎鎺ュ彛锛夈€?3. 鐢ㄧ洰鏍?Key 鍋氫竴娆℃渶灏忚璇佹祴璇曪紝渚嬪妯″瀷鍒楄〃鎴栧緢鐭殑娑堟伅璇锋眰銆?4. 鍙褰?Key 鏄惁瀛樺湪鍜岃璇佺粨鏋滐紝涓嶈鎶婄湡瀹?Key 鍐欏叆鏃ュ織銆佹埅鍥炬垨 issue銆?5. 纭 `~/.codex/config.toml` 宸叉湁澶囦唤锛屼究浜庢竻闄?API 妯″紡鍚庡洖婊氥€?
鍦ㄧ鐞嗗伐鍏风殑鈥滀腑杞敞鍏モ€濋〉闈細

1. 纭宸茬粡妫€娴嬪埌 ChatGPT 鐧诲綍鐘舵€併€?2. 娣诲姞涓€涓垨澶氫釜涓浆閰嶇疆锛屽～鍐?Base URL 鍜?Key銆?3. 閫夋嫨褰撳墠閰嶇疆骞跺簲鐢ㄤ腑杞敞鍏ャ€?4. 鍚姩 `Codex++`銆?
Codex++ 浼氬湪 `~/.codex/config.toml` 涓啓鍏ョ被浼奸厤缃細

```toml
model_provider = "CodexPlusPlus"

[model_providers.CodexPlusPlus]
name = "CodexPlusPlus"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

濡傛灉闇€瑕佸洖鍒板畼鏂圭櫥褰曟€侊紝鍦ㄢ€滀腑杞敞鍏モ€濋〉闈㈢偣鍑绘竻闄?API 妯″紡鍗冲彲绉婚櫎 `OPENAI_API_KEY` 鐩稿叧閰嶇疆骞跺垏鍥炲畼鏂?ChatGPT 鐧诲綍妯″紡銆?
## 澧炲己鍔熻兘

澧炲己鍔熻兘鍦ㄧ鐞嗗伐鍏蜂腑缁熶竴寮€鍏炽€傞粯璁ゅ紑鍚寮烘敞鍏ワ紱鍏抽棴鍚庝笉浼氭敞鍏?Codex++ 鑿滃崟鍜岃剼鏈€?
濡傛灉鍚敤涓浆娉ㄥ叆妯″紡锛屾彃浠跺叆鍙ｈВ閿佸拰寮哄埗瀹夎涓嶅啀闇€瑕侊紝鐣岄潰浼氭彁绀衡€滀腑杞敞鍏ユā寮忎笅鏃犻渶寮€鍚€濄€備細璇濆垹闄ゃ€佸鍑恒€佺Щ鍔ㄣ€乀imeline銆佹櫘閫氭帹鑽愬拰鐢ㄦ埛鑴氭湰绛夊寮轰粛鍙户缁娇鐢ㄣ€?
## 鎺ㄨ崘鍐呭

鎺ㄨ崘鍐呭鏉ヨ嚜杩滅▼鏅€氭帹鑽愬垪琛細

```text
https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool-Ad-List/main/ads.json
https://cdn.jsdelivr.net/gh/DamonZS/Claude-Codex-Pro-Tool-Ad-List@main/ads.json
```

璇锋眰鏃朵細鑷姩杩藉姞 `?v=鏃堕棿鎴砢 缁曞紑 CDN 鏃х紦瀛樸€傛櫘閫氭帹鑽愬姞杞芥參涓嶄細褰卞搷鍚庣杩炴帴鐘舵€併€?
## 鑷姩鏇存柊涓庡畨瑁呭寘

Codex++ 閫氳繃 GitHub Release 鍙戝竷瀹夎鍖呫€俉indows 浼氱敓鎴?NSIS 瀹夎绋嬪簭锛宮acOS 浼氱敓鎴?Intel x64 鍜?Apple Silicon arm64 涓や釜 DMG銆?
绠＄悊宸ュ叿鐨勨€滃叧浜庘€濋〉鍙互妫€鏌ュ苟鍚姩鏇存柊銆傞潤榛樺惎鍔ㄥ櫒鍙戠幇鏂扮増鏈椂浼氭媺璧风鐞嗗伐鍏峰苟杩涘叆鏇存柊鎻愮ず銆?
## 鏁版嵁浣嶇疆

- Codex 閰嶇疆锛歚~/.codex/config.toml`
- Codex 鐧诲綍鐘舵€侊細`~/.codex/auth.json`
- Codex 鏈湴鏁版嵁搴擄細浼樺厛璇诲彇 `~/.codex/sqlite/*.db`锛屾棫鐗堝洖閫€鍒?`~/.codex/state_5.sqlite`
- Codex++ 鐘舵€佷笌鏃ュ織锛歚~/.codex-session-delete/`
- Provider 鍚屾澶囦唤锛歚~/.codex/backups_state/provider-sync`

## 甯歌闂

### Codex++ 鑿滃崟娌″嚭鐜?
纭鏄粠 `Codex++` 鍏ュ彛鍚姩锛岃€屼笉鏄師鐗?Codex銆備篃鍙互鎵撳紑绠＄悊宸ュ叿鐨勨€滆瘖鏂€濆拰鈥滄棩蹇椻€濋〉闈㈡煡鐪嬫敞鍏ョ姸鎬併€?
### 鎻掍欢鍐呮樉绀哄悗绔繛涓嶄笂

鍏堝湪娴忚鍣ㄦ垨 PowerShell 閲屾祴璇曪細

```powershell
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:57321/backend/status -Body "{}" -ContentType "application/json"
```

濡傛灉鎺ュ彛姝ｅ父锛屼絾鎻掍欢浠嶆樉绀鸿秴鏃讹紝閫氬父鏄?Codex 椤甸潰閲岀殑 CDP bridge 鎴栬剼鏈紦瀛橀棶棰樸€傞噸鍚?Codex++锛屾垨鍦ㄧ鐞嗗伐鍏烽噷鏌ョ湅鏃ュ織涓殑 `renderer.script_loaded`銆乣bridge.request`銆乣bridge.response`銆?
### Upstream worktree 鍜?Codex 鍘熺敓鍒涘缓鏈変粈涔堝尯鍒?
Codex++ 鐨?Upstream worktree 鍔熻兘绛変环浜庡厛鏇存柊杩滅鍒嗘敮锛屽啀鎵ц锛?
```bash
git worktree add -b <new-branch> <worktree-path> upstream/<base-branch>
```

杩欐牱鏂?worktree 浠庢渶鏂扮殑杩滅璺熻釜鍒嗘敮寮€濮嬶紝鑰屼笉鏄粠褰撳墠浼氳瘽鎵€鍦ㄧ殑鏈湴 HEAD 寮€濮嬨€傚鏋?Codex++ 鏃犳硶瀹夊叏璇嗗埆褰撳墠 Codex 鐗堟湰鐨勫師鐢?worktree 鍒涘缓琛ㄥ崟锛岃浠?Codex++ 鑿滃崟涓墜鍔ㄥ～鍐欎粨搴撹矾寰勩€佸垎鏀悕銆亀orktree 璺緞銆乺emote 鍜?base branch銆?
### macOS 鎻愮ず鏃犳硶鎵撳紑鎴栧凡鎹熷潖

褰撳墠瀹夎鍖呮湭绛惧悕/鏈叕璇佹椂锛宮acOS Gatekeeper 鍙兘鎷︽埅锛屽嚭鐜扳€滃凡鎹熷潖锛屾棤娉曟墦寮€鈥濈殑鎻愮ず锛?
![macOS 鎻愮ず Codex++ 绠＄悊宸ュ叿宸叉崯鍧廬(docs/images/macos-damaged-warning.png)

濡傛灉閬囧埌璇ユ彁绀猴紝鍙互鍦ㄧ粓绔墽琛屼笅闈袱鏉″懡浠わ紝瑙ｉ櫎鑻规灉绯荤粺鐨勫畨鍏ㄩ殧绂婚檺鍒讹細

```bash
sudo xattr -rd com.apple.quarantine /Applications/Codex++\ 绠＄悊宸ュ叿.app
sudo xattr -rd com.apple.quarantine /Applications/Codex++.app
```

鎵ц鍚庨噸鏂版墦寮€ `Codex++` 鎴?`Codex++ 绠＄悊宸ュ叿` 鍗冲彲銆?
### macOS Intel 鑳界敤鍚?
鍙互銆俁elease 浼氬垎鍒彁渚?`macos-x64.dmg` 鍜?`macos-arm64.dmg`銆侷ntel Mac 涓嬭浇 x64 鍖咃紝Apple Silicon 涓嬭浇 arm64 鍖呫€?
## 寮€鍙?
```bash
# 鍓嶇妫€鏌?cd apps/claude-codex-pro-manager
npm install
npm run check
npm run vite:build

# Rust 妫€鏌?cd ../..
cargo fmt --check
cargo test
cargo build --release
```

涓昏缁撴瀯锛?
```text
apps/
  claude-codex-pro-launcher/          闈欓粯鍚姩鍏ュ彛
  claude-codex-pro-manager/           Tauri 绠＄悊宸ュ叿
assets/inject/
  renderer-inject.js            娉ㄥ叆鍒?Codex 娓叉煋绔殑澧炲己鑴氭湰
crates/
  claude-codex-pro-core/              鍚姩銆佹敞鍏ャ€侀厤缃€佹洿鏂般€佸畨瑁呫€佹ˉ鎺ョ瓑鏍稿績閫昏緫
  claude-codex-pro-data/              浼氳瘽鏁版嵁銆佸鍑恒€丳rovider 鍚屾
scripts/installer/
  windows/CodexPlusPlus.nsi     Windows NSIS 瀹夎鍖?  macos/package-dmg.sh          macOS DMG 鎵撳寘
```

## 鍙嬫儏閾炬帴

- [LINUX DO](https://linux.do)

## 璇存槑

Codex++ 鏄閮ㄥ寮哄伐鍏凤紝涓嶄慨鏀?Codex App 鍘熷鏂囦欢銆侰odex App 鏇存柊鍚庯紝濡傛灉椤甸潰缁撴瀯鍙樺寲锛屽彲鑳介渶瑕佹洿鏂版敞鍏ヨ剼鏈€?
