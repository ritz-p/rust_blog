# Cloudflare Deployment Guide

このドキュメントは、`Cloudflare のサービスだけで完結して公開する` 前提でまとめています。

結論から言うと、今の `Rocket + SeaORM + SQLite ファイル` を `prod/Dockerfile` のまま Cloudflare に置く手順ではありません。Cloudflare 完結で本番運用するなら、実際の着地点は次です。

- 配信: Cloudflare Workers Static Assets
- 動的処理: Cloudflare Workers
- データ: Cloudflare D1
- 画像などの大きなファイルが増えるなら: Cloudflare R2

## 結論

このリポジトリを Cloudflare 完結で出す場合、公開手順は次の 2 段階になります。

1. アプリを Cloudflare 向けの構成へ寄せる
2. Workers + D1 へデプロイする

いまのままの Docker イメージをそのまま本番に載せる方式は、Cloudflare 完結の本命ではありません。

理由は単純で、今の構成は次を前提にしているからです。

- Rocket の常駐 HTTP サーバー
- SQLite ファイルを直接開く運用
- 起動時 migration
- Markdown を DB に seed してから配信

Cloudflare 側で素直に運用したいなら、`SQLite ファイル` ではなく `D1` を使う前提へ寄せる必要があります。

## なぜこの形になるか

### Workers / Static Assets は相性が良い

Cloudflare Workers は動的処理を持てて、Static Assets は HTML / CSS / JS / 画像の配信をまとめて扱えます。

このブログ用途なら、次の分離がいちばん自然です。

- 記事ページや一覧ページの HTML は Workers が返す
- 画像や favicon は Static Assets か R2 から返す
- 記事、タグ、カテゴリなどのデータは D1 に置く

### Containers は今回の本命にしない

Cloudflare Containers は既存コンテナを動かせますが、2026-05-01 時点では Beta です。さらに、公式 FAQ ではディスクは ephemeral で、sleep 後の再起動時は fresh disk になると案内されています。

つまり、今の `SQLite をコンテナ内ファイルとして持つ` 運用は、そのままでは本番の永続データ置き場になりません。

そのため、Cloudflare 完結の本筋は `Workers + D1` です。

## このリポジトリで必要な変更

Cloudflare 完結にするには、少なくとも次の変更が必要です。

### 1. Rocket 常駐サーバー前提をやめる

今は `core/src/main.rs` で Rocket を起動していますが、Workers では `fetch(request, env)` で応答する形に寄せる必要があります。

対応方針:

- ルーティングを Workers 側へ移す
- HTML 生成ロジックは再利用できる形に切り出す
- HTTP サーバー起動前提のコードを減らす

### 2. SQLite ファイル前提をやめる

今の `DATABASE_URL=sqlite://...` 前提をやめて、D1 を使う設計に寄せます。

対応方針:

- SeaORM の利用継続可否を検討する
- 難しければ D1 へ直接 SQL を投げる層を作る
- migration は D1 向け SQL として管理する

### 3. seed を D1 向けに変える

今の seed は SQLite に対して実行する作りです。Cloudflare 完結にするなら、Markdown から D1 へ投入する流れに変える必要があります。

対応方針:

- `content/articles/*.md` を読む処理は流用する
- 出力先を SQLite ファイルではなく D1 にする
- 初回投入用コマンドを `wrangler d1 execute` または seed 用 Worker / スクリプトに寄せる

### 4. 画像や静的ファイルの置き場を整理する

現状 `content/image` や `content/icon` をアプリに同梱しています。Cloudflare 完結なら次のどちらかです。

- 配布物に含めて Static Assets から配信する
- 運用上分けたいなら R2 に置く

## 推奨アーキテクチャ

Cloudflare 完結で現実的なのは次の構成です。

- `Worker`
  - `/`
  - `/posts/:slug`
  - `/tags`
  - `/tag/:slug`
  - `/categories`
  - `/category/:slug`
- `D1`
  - articles
  - tags
  - categories
  - relation tables
- `Static Assets`
  - CSS
  - JS
  - favicon
  - 固定画像

記事本文が D1 に入り、Worker が HTML を返す構成です。

## 実際の手順

### 1. Cloudflare アカウントと Workers Paid plan を用意する

今回の前提では次を使う想定です。

- Workers
- D1
- 必要なら R2

Containers を検証用途で使うなら Workers Paid plan が必要です。Workers + D1 本体でも、商用運用前提なら Paid plan を前提にした方がよいです。

### 2. Workers プロジェクトを追加する

このリポジトリに Cloudflare 向けのアプリ層を追加します。

最低限必要なもの:

- `wrangler.toml`
- Worker エントリポイント
- D1 binding
- Static Assets の出力先

設定イメージ:

```toml
name = "rust-blog"
main = "worker/index.js"
compatibility_date = "2026-05-01"

[[d1_databases]]
binding = "DB"
database_name = "rust-blog-db"
database_id = "REPLACE_WITH_REAL_ID"

[assets]
directory = "./dist"
binding = "ASSETS"
```

ここでの `dist` は、静的ファイルの配置先です。HTML をすべて静的化する想定ではなくても、画像や CSS の置き場として使えます。

### 3. D1 データベースを作る

Cloudflare 側で D1 を作成します。

例:

```bash
wrangler d1 create rust-blog-db
```

作成後に返る `database_id` を `wrangler.toml` に反映します。

### 4. schema を D1 用に作る

今の migration をそのまま流すのではなく、D1 で実行する SQL を整理します。

手順:

1. 現在のテーブル定義を確認する
2. D1 向けの `schema.sql` を作る
3. ローカル D1 かリモート D1 に適用する

適用例:

```bash
wrangler d1 execute rust-blog-db --file=./cloudflare/schema.sql
```

### 5. seed 手順を D1 向けに作る

初回データ投入は必須です。ブログは Markdown から記事を読み込む構成なので、次のどちらかに寄せます。

- Rust の seed ロジックを D1 書き込み対応に変える
- D1 へ投入する JSON / SQL を生成して `wrangler d1 execute` で流し込む

運用として分かりやすいのは次です。

1. Markdown を読む
2. front matter を解釈する
3. 記事、タグ、カテゴリを SQL か JSON に落とす
4. D1 に投入する

### 6. HTML 生成を Worker に寄せる

ページ描画は次のどちらかです。

- Worker が毎回 D1 を読んで HTML を返す
- 更新時に一部ページを事前生成して Static Assets に寄せる

まずはシンプルに、Worker が D1 を読んで HTML を返す形でよいです。

必要な処理:

- index 一覧取得
- slug から記事取得
- tag / category 一覧取得
- Askama 相当のテンプレート描画方法を決める

Rust のテンプレート資産を残したい場合は、Cloudflare 向け Rust 実装に寄せる追加作業が必要です。最短では Worker 側を TypeScript / JavaScript で切り出す方が単純です。

### 7. 静的ファイルを `dist` に集める

少なくとも次を `dist` に置きます。

- favicon
- `/image/*`
- `/icon/*`
- CSS
- 必要な JS

Cloudflare Workers Static Assets を使うと、これらは Cloudflare 側でキャッシュ配信されます。

### 8. ローカルで Workers と D1 を確認する

デプロイ前にローカル確認します。

```bash
wrangler dev
```

確認項目:

- `/` が表示できる
- 記事詳細が slug で引ける
- タグ、カテゴリの一覧が取れる
- 画像や favicon が読める

### 9. Cloudflare にデプロイする

デプロイは `wrangler deploy` を使います。

```bash
wrangler deploy
```

Static Assets も Worker も同時に Cloudflare へ反映されます。

### 10. カスタムドメインをつなぐ

Cloudflare 側で Workers の route か custom domain を設定します。

例:

- `example.com/*`
- `www.example.com/*`

同じ Cloudflare アカウント内で完結するため、DNS, TLS, CDN, WAF もそのまま Cloudflare 上で管理できます。

### 11. 本番確認をする

確認項目:

- `/` が 200
- `/posts/<slug>` が 200
- タグ、カテゴリ一覧が引ける
- 画像が 404 にならない
- D1 へ期待通りデータが入っている
- Cloudflare のキャッシュやルーティングが意図通り

## 更新手順

記事更新の流れは次です。

1. `content/articles` を更新する
2. seed を再実行して D1 を更新する
3. 必要なら静的ファイルを再ビルドする
4. `wrangler deploy` で再反映する

画像だけ差し替えるなら Static Assets 側だけの更新で済む場合もあります。

## Containers を使う案

Cloudflare 完結という意味では、Containers を使って既存 Docker イメージを Cloudflare 上で動かす案もあります。

ただし、2026-05-01 時点では次の理由でこのブログの本番構成としては弱いです。

- Beta
- ディスクが永続ではない
- SQLite ファイルをそのまま本番データにしづらい

そのため、このリポジトリでは Containers は次の用途までに留めるのが妥当です。

- 一時的な移行検証
- 既存 Rocket 実装の動作確認
- 本番移行前の段階的検証

## このリポジトリに対する実務上のすすめ方

Cloudflare 完結を本当に目指すなら、順番は次です。

1. D1 schema を作る
2. Markdown seed を D1 向けに作り替える
3. Worker ルーティングを実装する
4. Static Assets 配信へ寄せる
5. `wrangler deploy` で本番化する

つまり、`デプロイ手順の変更` だけではなく、`Cloudflare 向けの実装変更` が先に必要です。

## 参考

- Workers overview: <https://developers.cloudflare.com/workers/>
- Static Assets: <https://developers.cloudflare.com/workers/static-assets/>
- D1 overview: <https://developers.cloudflare.com/d1/>
- Containers overview: <https://developers.cloudflare.com/containers/>
- Containers FAQ: <https://developers.cloudflare.com/containers/beta-info/>
