# Cloudflare Deployment Guide

このブログを Cloudflare で動かすときは、まず「どこまでを Cloudflare に任せるか」を決める必要があります。

現状のアプリは以下の構成です。

- Rust + Rocket の常駐サーバー
- SeaORM
- SQLite
- Markdown を seed して DB に投入

そのため、Cloudflare Workers / Pages にそのまま載せることはできません。Rocket サーバーを起動し続ける前提の実装だからです。

## 結論

このリポジトリに対して現実的な選択肢は次の 3 つです。

### 1. いちばん早い: アプリは別のコンテナ基盤で動かし、Cloudflare は DNS / CDN / WAF に使う

おすすめ度: 高

向いているケース:

- まずは今のコードをほぼそのまま公開したい
- Rocket を維持したい
- SQLite か別 DB をそのまま使いたい

構成イメージ:

- アプリ本体: Fly.io / Render / Railway / VPS / ECS など
- 公開ドメイン: Cloudflare DNS
- HTTPS / CDN / WAF: Cloudflare Proxy

この方式なら、このアプリの変更は最小限ですみます。

必要な作業:

1. コンテナで `cargo run -p rust_blog` ではなく release バイナリを起動する
2. `DATABASE_URL` を本番環境向けに設定する
3. `blog.db` を永続ボリュームに置く
4. Cloudflare 側で対象ホスト名を Proxy 有効で向ける

注意点:

- SQLite を使い続ける場合は、単一インスタンス運用が基本です
- 複数台構成やスケールアウトには向きません

### 2. Cloudflare らしく運用する: 静的サイトにして Cloudflare Pages へ載せる

おすすめ度: 高

向いているケース:

- ブログが基本的に読み取り専用
- 更新はビルド時反映でよい
- 一番安定してシンプルに運用したい

構成イメージ:

- Markdown から HTML をビルド
- 生成した静的ファイルを Cloudflare Pages に配置
- 画像や CSS / JS も静的配信

このブログは記事ソースが `content/articles/*.md` にあるので、長期的にはこの形がかなり相性がよいです。

ただし現状は「Markdown -> SQLite に seed -> Rocket が HTML を返す」構成なので、以下のどちらかの対応が必要です。

- 静的 HTML を吐くビルドコマンドを新しく作る
- 別の静的サイトジェネレーター構成へ寄せる

必要な実装の方向:

1. ルーティングごとの HTML を事前生成する export コマンドを作る
2. `/`, `/posts/<slug>`, `/tags`, `/tag/<slug>`, `/categories`, `/category/<slug>` を出力する
3. CSS / JS / 画像を `dist/` にまとめる
4. Cloudflare Pages のビルド成果物を `dist` にする

### 3. Cloudflare に全部寄せる: Workers + D1 へ作り替える

おすすめ度: 中

向いているケース:

- Cloudflare に統一したい
- サーバーレス運用に寄せたい
- 実装変更コストを受け入れられる

必要な変更:

- Rocket をやめて Workers 向け実装に置き換える
- SQLite ファイル直置きをやめて D1 などへ移行する
- ルーティング、テンプレート描画、DB アクセスを Workers 前提で再設計する

これは「デプロイ設定の追加」で済まず、アプリ構成の変更になります。

## このリポジトリに対するおすすめ

優先度順では次のどちらかです。

### まず公開したいなら

Cloudflare はフロントに使い、アプリ本体はコンテナ対応のホスティングへ出すのが最短です。

### Cloudflare に綺麗に載せたいなら

Rocket アプリのまま無理に Workers へ載せるより、静的出力を追加して Cloudflare Pages に置く方が相性がよいです。

ブログ用途なら、この方が運用コストもかなり低くなります。

## 具体的な進め方

### 最短公開ルート

1. 本番用 Dockerfile を用意する
2. `cargo build --release -p rust_blog` でビルドする
3. `DATABASE_URL=sqlite:///data/blog.db` のように永続領域を使う
4. デプロイ先で `./target/release/rust_blog` を起動する
5. Cloudflare DNS でドメインを向ける
6. Cloudflare Proxy, TLS, Cache Rules を有効化する

### Pages へ寄せるルート

1. `export` 用の Rust バイナリを追加する
2. Markdown / SQLite / テンプレートから静的 HTML を出力する
3. 出力先を `dist/` に統一する
4. Cloudflare Pages の Build command を export コマンドにする
5. Output directory を `dist` にする

## 今のまま Cloudflare Workers に載らない理由

- Rocket は常駐 HTTP サーバー前提
- `main` で DB 接続を張って Rocket を起動している
- SQLite ファイル運用を前提にしている
- Cloudflare Pages は静的配信向け
- Workers はリクエストハンドラ型の実装に寄せる必要がある

## 参考にした Cloudflare 公式ドキュメント

- Workers overview: <https://developers.cloudflare.com/workers/>
- Pages overview: <https://developers.cloudflare.com/pages/>
- D1 overview: <https://developers.cloudflare.com/d1/>
- Containers overview: <https://developers.cloudflare.com/containers/>

## 次にやるとよいこと

このリポジトリなら、次のどちらかを選ぶのがおすすめです。

- すぐ公開したい: 本番用 Dockerfile とデプロイ設定を追加する
- Cloudflare にきれいに載せたい: 静的 export 機能を追加する

後者を選ぶなら、このブログ向けに `dist/` を生成する export コマンドまで実装できます。
