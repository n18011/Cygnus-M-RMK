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

## ビルドと書き込み

Rust、`thumbv7em-none-eabihf` ターゲット、`cargo-make`、`clang/libclang` を準備したうえで実行します。

Ubuntu系では、事前に `sudo apt install clang libclang-dev` を実行してください。`nrf-sdc` の
バインディング生成に libclang が必要です。

```sh
cd Cygnus-M-RMK
rustup target add thumbv7em-none-eabihf
cargo install --force cargo-make
cargo make uf2 --release
```

生成された UF2 のうち `central` を右側（`Cygnus_R`）へ、`peripheral` を左側（`Cygnus_L`）へ書き込みます。
XIAO のブートローダーが UF2 に対応していることを確認してください。RMK 0.7 以降へ
移行する際は BLE スタックが変わるため、ZMK へ戻す場合にブートローダーの再書き込みが
必要になることがあります。

## Vial Web

Vial Web は USB ケーブルを右側の Central (`Cygnus_R`) に接続して使用します。
初回接続時は Vial のロック解除操作で、右側の `row=0,col=7` と `row=0,col=8` のキーを
同時に押してください。これは `keyboard.toml` の `unlock_keys` に設定した解除コンボです。
解除後のキーマップ変更は内蔵ストレージへ保存されます。
既存の保存済みキーマップがある場合はデフォルト設定より優先されるため、スクロールキーを
`LT(scroll,Semicolon)` に設定するか、キーマップを初期化してください。

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
- 元リポジトリの RGB adapter / Studio RPC は RMK 版では未設定です。

RMK の分割キーボード設定は [公式ドキュメント](https://rmk.rs/docs/configuration/split_keyboard)、
PMW3610 の設定は [入力デバイスのドキュメント](https://rmk.rs/docs/configuration/input_device/pmw3610)
に準拠しています。
