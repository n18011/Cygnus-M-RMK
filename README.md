# Cygnus-M (RMK)

[n18011/Cygnus-M](https://github.com/n18011/Cygnus-M) を RMK 0.8.2 向けに移植したファームウェアです。

## 構成

- Seeed XIAO nRF52840 を左右に 1 枚ずつ使用する BLE 分割キーボード
- 右側（元の `Cygnus_R`）が Central、左側（元の `Cygnus_L`）が Peripheral
- 4 行・左右合計 51 キー、7 レイヤー、BLE プロファイル 5 個
- 左側に EC11 ロータリーエンコーダー、右側に PMW3610 トラックボール
- 元の ZMK キーマップ、コンボ、モールス入力、手動マウス/スクロールレイヤーを移植

レイアウトとキーマップは [keyboard.toml](keyboard.toml)、Vial 用の行列定義は
[vial.json](vial.json) にあります。Vial の物理レイアウトは ZMK 版の
`Cygnus.dtsi` にある親指キーの位置・角度を反映しています。

## 配線

| 側 | 行ピン | 列ピン | その他の入力 |
| --- | --- | --- | --- |
| Central / 右 (`Cygnus_R`) | `P0_03`, `P0_28`, `P0_29`, `P1_11` | `P1_15`, `P1_14`, `P1_13`, `P1_12`, `P0_10`, `P1_10` | PMW3610 SCK=`P0_05`, SDIO=`P0_04`, CS=`P0_09`, MOTION=`P0_02` |
| Peripheral / 左 (`Cygnus_L`) | `P0_03`, `P0_28`, `P0_29`, `P1_11` | `P1_15`, `P1_14`, `P1_13`, `P1_12`, `P0_10`, `P0_09`, `P0_04` | EC11 A=`P0_05`, B=`P0_02` |

ピン名は XIAO BLE の D ピンではなく、nRF52840 の GPIO 名で記述しています。
元の ZMK 配線では `col2row` だったため、RMK でも `row2col = false` にしています。

## RGB LED

ZMK の `zmk-rgbled-widget` の状態表示を、RMK の `Controller` / `PollingController` で
3本のGPIOへ移植しています。RGB LEDはactive-lowとして、次のピンを使用します。

| 色 | GPIO | 点灯レベル |
| --- | --- | --- |
| Red | `P0_26` | Low |
| Green | `P0_30` | Low |
| Blue | `P0_06` | Low |

ファームウェアは初期状態をHigh（消灯）にして、RMKの`CONTROLLER_CHANNEL`を50 ms周期で
購読・更新します。

| 状態 | 表示 |
| --- | --- |
| Battery 0% / 1–19% / 20–79% / 80%以上 | Magenta / Red / Yellow / Green |
| Batteryが1–5%へ遷移 | Redを3回点滅 |
| Central: BLE advertising / connected / disconnected | Yellow / Blue / Red |
| Central: USB接続 | Cyan |
| Peripheral: Centralとのsplitリンク切断 / 接続 | Red / Blue |
| CentralのLayer Nへの変更 | CyanをN回点滅（Base layer 0は点滅なし） |
| Charging / Sleep | Green / 消灯 |

LayerイベントはRMKのsplit同期でPeripheralにも届きますが、ZMK widgetと同じくPeripheralでは
無視します。RGBの状態機械は [src/rgb_led_state.rs](src/rgb_led_state.rs)、GPIO Controllerは
[src/rgb_led.rs](src/rgb_led.rs) にあります。active-highのLED基板を使う場合は、GPIO初期値と
`apply_color`の極性を合わせて変更してください。

## ビルド

Rust、`thumbv7em-none-eabihf` ターゲット、`cargo-make`、`clang/libclang` を準備したうえで実行します。

Ubuntu系では、事前に `sudo apt install clang libclang-dev` を実行してください。`nrf-sdc` の
バインディング生成に libclang が必要です。

```sh
cd Cygnus-M-RMK
rustup target add thumbv7em-none-eabihf
cargo install --force cargo-make
cargo make bin --release
```

`cargo make bin --release` は dfu-util 用の生バイナリを出します。

- `Cygnus-M-RMK-central.bin` … 右側 Central
- `Cygnus-M-RMK-peripheral.bin` … 左側 Peripheral

この二つのリンクアドレスは `memory.x` の ACTIVE `0x7000` です。これは rmk-boot の
標準レイアウト（非 defmt の swap、および同じ ACTIVE 原点を使う非 defmt の noswap）向けの
**候補**です。`1209:0001` という USB ID だけでは、実機が swap / noswap / `dfu_ext` /
defmt のどれかを証明しません。defmt ブートローダは 32 KiB で ACTIVE が後ろへずれ、
`dfu_ext` は外部 SPI を DFU に使います。どちらもこの `memory.x` とは違います。

Makefile は引き続き UF2 と HEX も生成できます。それらを dfu-util の入力にしてはいけません。
dfu-util に渡すのは上記 `.bin` だけです。

### 書き込み前提

実機のブートローダ配置が標準の非 defmt rmk-boot だと確認できるまで、書き込まないでください。
配置が違うと ACTIVE の先頭が `0x7000` ではなく、起動しないか、noswap では直接 ACTIVE を
上書きします。swap ならブートローダがロールバックできることがありますが、それは保証ではありません。

前提を自分で受け入れた場合だけ、ブートローダの USB DFU（`1209:0001`、alt 0）に対して次の形です。
アプリの `4c4b:4643` には DFU インターフェースがありません。RMK 0.8.2 には `dfu_nrf` がありません。

```sh
# 右側。Vial の Security → Reboot to bootloader で 1209:0001 が出ている間
dfu-util -d 1209:0001 -a 0 -D Cygnus-M-RMK-central.bin
```

左側の peripheral ファームは USB デバイスを出しません。Vial のブートローダ操作がリセットするのは
右側の Central だけです。左側を更新するには、左側のチップ自身の rmk-boot が DFU になっている
必要があります。rmk-boot の文書ではリセットピンの二度押しです。左側に同じブートローダがあるかは
未確認です。

swap モードでは、起動したアプリが状態ページ先頭の `SWAP_MAGIC`（`0xF0F0F0F0`）を
`BOOT_MAGIC`（`0xD0D0D0D0`）へ変えないと、次のリセットでロールバックします。
RMK 0.8.2 にはガイドの `dfu_nrf` / `mark_booted()` がないため、ストレージ初期化のあと
MPSL の flash ドライバで同じ 4 バイトだけを書きます。消去はしません。`0xF0` 以外は
書き換えません。初期化前に落ちた場合は magic が残るので、ブートローダは戻せます。
defmt ビルドの状態ページは `0x6000` ではないため、この確認は届きません。
このビルドは実行中アプリの DFU（`4c4b:4643`）を出しません。更新入口は rmk-boot の
`1209:0001` です。

## Vial Web

Vial Web は USB ケーブルを右側の Central (`Cygnus_R`) に接続して使用します。
初回接続時は Vial のロック解除操作で、右側の `row=0,col=7` と `row=0,col=8` のキーを
同時に押してください。これは `keyboard.toml` の `unlock_keys` に設定した解除コンボです。
解除後のキーマップ変更は内蔵ストレージへ保存されます。
既存の保存済みキーマップがある場合はデフォルト設定より優先されるため、スクロールキーを
`LT(scroll,Semicolon)` に設定するか、キーマップを初期化してください。

## ストレージの初期化

RMK は Vial のキーマップ、動作設定、BLE bond、split peer 情報などを XIAO の内部 Flash に
保存します。標準レイアウトでは末尾の `0xF8000` から 32 KiB（8 セクタ）です。アプリ本体の
DFU 上書きはこの領域を通常消しません。以前の `0xA0000` にあった保存値は読みません。

このプロジェクトのRMK 0.8.2ベンダー版には、現在のファームウェアの build hash と保存値が
異なるとストレージを再初期化する修正を適用しています。新しい `.bin` を左右へ書き込んで起動すると、
各側が一度初期化されます。BLE bond も消えるため、初期化後はホストとのペアリングをやり直してください。

確実に全消去する場合は、公式の `clear_storage` 手順を使います。

1. `keyboard.toml` の `[storage]` で `clear_storage = true` にして、`cargo make bin --release` を実行する。
2. 生成した `central` と `peripheral` の `.bin` を、それぞれ右側・左側のブートローダ DFU へ書き込み、両方を一度起動する。
3. `clear_storage = false` に戻して再ビルド・再書き込みする。trueのままだと毎回起動時に消去されます。

Vial の EEPROM Reset（設定初期化）が使える場合も同じストレージを消去できます。このRMK版では
消去後に自動再起動するため、デフォルトキーマップを読み直します。公式の仕様は
[RMK Storage](https://rmk.rs/main/docs/configuration/storage) と
[RMK FAQ](https://rmk.rs/main/docs/getting_started/faq) を参照してください。

## 移植時の差分

- BLE では PMW3610 が生成する移動レポートの方が送信より速く、古いカーソル位置が
  キューに滞留しやすいため、`vendor/rmk-0.8.2` に純粋な X/Y 移動レポートを合成する
  パッチを適用しています。ボタン、ホイール、パン、ボタン解放レポートは合成対象外です。
- RMK 0.8.2 の PMW3610 は通常レイヤーではカーソル入力として扱い、`scroll` レイヤー（layer 4）
  が有効な間は X/Y を64カウント単位の縦ホイール／横パンへ変換します。斜め入力では
  垂直スクロールを優先します。Base の JIS `;` はタップで `;`、ホールドで `scroll` レイヤーを有効にします。
- PMW3610 の X 軸は基板上のセンサー向きに合わせ、`invert_x = false` としています。
- RMK 0.8.2 の `async_matrix` は現行ツールチェーンで型不整合になるため有効化せず、通常の
  マトリクススキャンを使用しています。キー配列・BLE分割・入力デバイスの動作には影響しません。
- `BT_CLR_ALL` は RMK の `User9`（`CLR_PEER`、長押し 5 秒で split peer の bond を消去）に割り当て、
  `BT0`～`BT4` は `User0`～`User4`、`BT_NEXT`/`BT_PREV` は `User5`/`User6` に移しました。
- ZMK の `&kt KP_N4` は RMK 0.8.2 に対応するキー・トグルがないため、通常の `Kp4` にしています。
- 元リポジトリの RGB adapter / Studio RPC は移植せず、3色GPIOの状態表示をRMK Controllerで
  自作しています。

RMK の分割キーボード設定は [公式ドキュメント](https://rmk.rs/docs/configuration/split_keyboard)、
PMW3610 の設定は [入力デバイスのドキュメント](https://rmk.rs/docs/configuration/input_device/pmw3610)
に準拠しています。
