# Export Bundle Usage

このドキュメントは、リリースタグごとに CI で生成した `rust-blog-export-tools-<version>-x86_64-unknown-linux-gnu.tar.gz` を別リポジトリで使って `dist/` を生成する手順をまとめたものです。

## 配布方針

export bundle は `master` への push ごとには配りません。  
`v*` 形式のタグを切ったときだけ CI で生成し、そのバージョンの artifact として扱います。

例:

- タグ: `v0.1.0`
- artifact: `rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`

## 含まれるもの

artifact には次が含まれます。

- `export`
- `migration`
- `blog_config.toml`
- `templates/`

`export` は単体バイナリではなく、上記ファイル群と同じディレクトリ構成で使う前提です。

## 前提

- Linux x86_64 環境で実行する
- `DATABASE_URL` を設定できる
- SQLite を使う場合は書き込み可能な配置先を使う
- 別リポジトリ側で記事データ投入元を用意する

## 推奨ディレクトリ構成

別リポジトリで、artifact 展開先を例えば `tools/rust-blog-export/` に置きます。

```text
your-static-site-repo/
├─ tools/
│  └─ rust-blog-export/
│     ├─ export
│     ├─ migration
│     ├─ blog_config.toml
│     ├─ templates/
│     └─ ...
├─ content/
│  ├─ icon/
│  └─ image/
├─ blog.db
└─ dist/
```

`content/` は bundle に含まれません。必要なら別リポジトリ側で管理します。

## パス解決ルール

`export` は次の環境変数で参照先を上書きできます。

- `RUST_BLOG_CONFIG_PATH`
- `RUST_BLOG_TEMPLATES_DIR`
- `RUST_BLOG_CONTENT_DIR`

未指定時は、まず現在の作業ディレクトリを見て、見つからなければ `export` バイナリの配置ディレクトリを見ます。  
別リポジトリで `content/` を bundle の外に置く場合は、`RUST_BLOG_CONTENT_DIR` を設定してください。

## 最小手順

1. 対象リリースの CI artifact を取得して展開する
2. `DATABASE_URL` を設定する
3. `migration up` を実行する
4. 必要な記事データを DB へ投入する
5. `export dist` を実行する

例:

```bash
mkdir -p tools
tar -xzf rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz -C tools
cd tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu
export DATABASE_URL="sqlite://../../blog.db?mode=rwc"
export RUST_BLOG_CONTENT_DIR="../../content"
./migration up
./export ../../dist
```

## SQLite を使う場合

SQLite を使うなら、別リポジトリ側で `blog.db` を管理します。

例:

```bash
touch blog.db
mkdir -p tools
tar -xzf rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz -C tools
cd tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu
export DATABASE_URL="sqlite://../../blog.db?mode=rwc"
export RUST_BLOG_CONTENT_DIR="../../content"
./migration up
./export ../../dist
```

## DB への記事投入について

この bundle には `seed` バイナリを含めていません。したがって、記事データ投入は別リポジトリ側で決める必要があります。

選択肢:

- このリポジトリ側で将来 `seed` も bundle に含める
- 別リポジトリ側で SQL を直接流す
- 別リポジトリ側で独自の import スクリプトを持つ

現状の bundle は、`export` 実行時点で必要なテーブルとデータが揃っていることを前提にしています。

## 出力先

`./export <path>` の `<path>` が `dist/` になります。

例:

```bash
./export ../../dist
```

この結果、別リポジトリ側の `dist/` に HTML, CSS, JS, `_headers`, `_redirects` が出力されます。  
`content/image` と `content/icon` が別リポジトリ側に存在する場合は、それらも `dist/` にコピーされます。

## Cloudflare へ載せる場合

別リポジトリ側では、生成された `dist/` だけをデプロイ対象にします。

想定フロー:

1. export bundle を取得
2. DB を用意して migration / data import を実行
3. `dist/` を生成
4. `dist/` を Cloudflare Pages または Workers Static Assets へデプロイ

## 注意点

- `export` は `templates/` を実行時に読む
- `blog_config.toml` も実行時に読む
- `content/image` と `content/icon` は bundle に含まれない
- 画像や icon が必要なら、別リポジトリ側で `content/image` と `content/icon` を用意し、必要に応じて `RUST_BLOG_CONTENT_DIR` を設定する

## 将来の改善候補

- `seed` バイナリも bundle に含める
- bundle を `.tar.gz` でまとめて release artifact 化する
- 実行時依存を埋め込んで、`export` 単体で完結する形へ寄せる
