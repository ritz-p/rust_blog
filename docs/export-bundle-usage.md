# Export Bundle Usage

CI で生成した export バンドルを、別の記事リポジトリで使って `dist/` を生成する手順です。

## 配布方針

`.github/workflows/static_export.yml` の **Export Tool Build** は `v*` タグの push と手動実行に対応します。`develop` への通常の push では生成しません。
手動実行する場合は `develop` またはリリースタグを選択してください。バンドル名には選択した ref 名が使われます。

`v0.1.0` の例:

- Actions artifact: `rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu`
- artifact 内のファイル: `rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`

## 含まれるもの

- `export`
- `migration`
- `seed`
- `templates/`

Linux x86_64 環境で実行します。`export` は `templates/` を実行時に読むため、バイナリだけを取り出さずに使います。
記事 Markdown、画像・アイコン、`blog_config.toml` はバンドルに含まれません。記事リポジトリ側で用意してください。リダイレクトを使う場合は `redirects.toml` も用意します。

タグ・カテゴリは各記事のフロントマターで指定します。`blog_config.toml` はサイト名・画像ディレクトリなどの共通設定を持ち、seed はこの設定ファイルを読みません。

## パス解決

以下のコマンドは、記事を管理するリポジトリのルートで実行します。

| 環境変数 | 用途 |
| --- | --- |
| `DATABASE_URL` | migration・seed・export が使う DB |
| `ARTICLE_PATH` | seed が読み込む記事ディレクトリ |
| `FIXED_CONTENT_PATH` | seed が読み込む固定ページディレクトリ |
| `RUST_BLOG_CONFIG_PATH` | export が読むブログ設定 |
| `RUST_BLOG_TEMPLATES_DIR` | export が読むテンプレート |
| `RUST_BLOG_CONTENT_DIR` | export がコピーする画像・アイコンの親ディレクトリ |

export の設定・テンプレート・コンテンツのパスを省略した場合は、現在の作業ディレクトリを探し、見つからなければバイナリの配置ディレクトリを探します。
`blog_config.toml` の `image_dir`・`icon_dir` は `RUST_BLOG_CONTENT_DIR` より優先され、相対パスは作業ディレクトリから解決します。バンドルのディレクトリへ移動せずに実行してください。

## SQLite から静的サイトを生成する

対象の Actions artifact をダウンロードし、中の `.tar.gz` を記事リポジトリのルートに置きます。
以下は `content/articles/`・`content/fixed_contents/` と `blog_config.toml` を用意した場合の例です。

```bash
(
set -eu
mkdir -p tools
tar -xzf rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu.tar.gz -C tools
export_tools_dir=tools/rust-blog-export-tools-v0.1.0-x86_64-unknown-linux-gnu
export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal" "$export_db_dir/blog.db-journal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export ARTICLE_PATH="content/articles"
export FIXED_CONTENT_PATH="content/fixed_contents"
export RUST_BLOG_CONFIG_PATH="blog_config.toml"
export RUST_BLOG_CONTENT_DIR="content"
export RUST_BLOG_REQUIRE_CREATED_AT=1
export RUST_BLOG_TEMPLATES_DIR="$export_tools_dir/templates"
"$export_tools_dir/migration" up
"$export_tools_dir/seed"
"$export_tools_dir/export" dist
)
```

`RUST_BLOG_REQUIRE_CREATED_AT=1` は記事の公開日時を必須にします。
更新日時もビルドごとに変えずに引き継ぐ場合は、seed に `RUST_BLOG_FILE_TIMESTAMPS=1` を設定してください。Git 管理下のコンテンツでは Git と完全な履歴が必要です。詳細は [更新日時](export-timestamps.md) を参照してください。

毎回新規の一時 DB へ migration・seed を実行するため、削除した Markdown が前回の DB から再公開されることはありません。既存の開発用 `blog.db` は変更せず、終了時に一時 DB を削除します。サブシェルの外へ環境変数の変更を残しません。
`export` 自体は Markdown を取り込みません。記事の更新時も migration・seed・export の順に実行します。

## 出力と公開

最後のコマンドの `dist` が出力先です。HTML・CSS・JavaScript・検索インデックス・`_headers`・`_redirects`・`robots.txt` と、設定した画像・アイコンを出力します。`public_url` を設定した場合は `sitemap.xml` も生成します。

出力先はディレクトリごと切り替えるため、Docker からホストへ出力するときは親ディレクトリをマウントします。失敗時の旧出力保持とプレビュー方法は [静的デプロイ](static-deploy.md) を参照してください。
別の記事リポジトリでは、生成した `dist/` だけを Cloudflare Pages や Workers Static Assets へデプロイします。DB やバンドルは公開対象に含めません。
