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
- `seed`
- `templates/`

`export` は単体バイナリではなく、上記ファイル群と同じディレクトリ構成で使う前提です。

## 前提

- Linux x86_64 環境で実行する
- `DATABASE_URL` を設定できる
- SQLite を使う場合は書き込み可能な配置先を使う
- 別リポジトリ側で記事データ投入元を用意する
- 別リポジトリ側で `blog_config.toml` を用意する（`[common]`・`[categories]`・`[tags]` テーブルが必要）

## 推奨ディレクトリ構成

別リポジトリで、artifact を `tools/` に展開します。以下は `v0.1.0` の例です。

```text
your-static-site-repo/
├─ tools/
│  └─ rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/
│     ├─ export
│     ├─ migration
│     ├─ seed
│     ├─ templates/
│     └─ ...
├─ content/
│  ├─ icon/
│  └─ image/
├─ blog_config.toml
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

以下の実行例はすべて、記事を管理するリポジトリのルートで実行します。
`blog_config.toml` の `image_dir`・`icon_dir` は `RUST_BLOG_CONTENT_DIR` より優先され、
相対パスは実行時の作業ディレクトリから解決されます。`content/image` などを指定した場合も、
バンドルのディレクトリへ移動せずに実行することで画像・アイコンを正しくコピーできます。

## 最小手順

1. 対象リリースの CI artifact を取得して展開する
2. export 専用の一時ディレクトリを作り、新規 DB を指す `DATABASE_URL` を設定する
3. `migration up` を実行する
4. `seed` で Markdown を DB へ投入する
5. `export dist` を実行する

例:

```bash
(
set -eu
mkdir -p tools
tar -xzf rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz -C tools
export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal" "$export_db_dir/blog.db-journal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export RUST_BLOG_CONTENT_DIR="content"
export ARTICLE_PATH="content/articles"
export FIXED_CONTENT_PATH="content/fixed_contents"
export CONFIG_TOML_PATH="blog_config.toml"
export RUST_BLOG_CONFIG_PATH="$CONFIG_TOML_PATH"
export RUST_BLOG_REQUIRE_CREATED_AT=1
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/migration up
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/seed
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/export dist
)
```

## SQLite を使う場合

SQLite は毎回新しい一時 DB を使い、終了時に削除します。
既存の `blog.db` を再利用すると、Markdown を削除した記事や固定ページが DB に残り、再公開されるためです。
`touch blog.db` では既存データは消えません。以下の手順は既存の開発用 DB に触れません。

例:

```bash
(
set -eu
mkdir -p tools
tar -xzf rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz -C tools
export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal" "$export_db_dir/blog.db-journal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export RUST_BLOG_CONTENT_DIR="content"
export ARTICLE_PATH="content/articles"
export FIXED_CONTENT_PATH="content/fixed_contents"
export CONFIG_TOML_PATH="blog_config.toml"
export RUST_BLOG_CONFIG_PATH="$CONFIG_TOML_PATH"
export RUST_BLOG_REQUIRE_CREATED_AT=1
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/migration up
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/seed
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/export dist
)
```

## DB への記事投入について

バンドルの `seed` を使用します。古いバンドルには含まれていないため、再ビルドしてください。
別リポジトリのルートから、次のように新規の一時 DB に migration・seed・export を順に実行します。

```bash
(
set -eu
export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal" "$export_db_dir/blog.db-journal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export ARTICLE_PATH="content/articles"
export FIXED_CONTENT_PATH="content/fixed_contents"
export CONFIG_TOML_PATH="blog_config.toml"
export RUST_BLOG_CONFIG_PATH="$CONFIG_TOML_PATH"
export RUST_BLOG_CONTENT_DIR="content"
export RUST_BLOG_REQUIRE_CREATED_AT=1
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/migration up
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/seed
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/export dist
)
```

`export` 自体は Markdown を取り込みません。上記の全実行例は毎回空の DB から生成するため、入力ディレクトリから除外した記事・固定ページは出力されません。
括弧内のサブシェルで実行することで、終了時に一時 DB を片付け、呼び出し元の作業ディレクトリや環境変数も維持します。

## 出力先

`./export <path>` の `<path>` が `dist/` になります。

例:

```bash
./tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu/export dist
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

- 実行時依存を埋め込んで、`export` 単体で完結する形へ寄せる
