# Rust Blog

Rocket・SeaORM を使うブログ。サーバー表示と静的 HTML 出力に対応しています。

## 構成

- `core/src/`: アプリケーション、エンティティ、seed、静的 export
- `migration/`: DB マイグレーション
- `content/articles/`: 記事 Markdown
- `content/fixed_contents/`: 固定ページ Markdown
- `templates/`: Tera テンプレート
- `core/assets/`: 静的アセット
- `blog_config.toml`: ブログ設定

## 開発

Docker を使用します。`web` サービスでは `DATABASE_URL=sqlite://./blog.db?mode=rwc` が設定されます。

```bash
docker compose up -d web
docker compose exec web sea-orm-cli migrate up
docker compose exec web cargo run -p rust_blog --bin seed
docker compose exec web cargo run -p rust_blog
```

[http://localhost:8888/](http://localhost:8888/) で確認できます。
`ARTICLE_PATH`・`FIXED_CONTENT_PATH` で seed の入力ディレクトリを変更できます。

エンティティの再生成:

```bash
docker compose exec web sea-orm-cli generate entity -o core/src/entity --with-serde both
```

テスト:

```bash
docker compose exec web cargo test -p rust_blog
```

## 静的出力

```bash
docker compose exec web cargo run -p rust_blog --bin export
docker compose up -d --force-recreate static
```

[http://localhost:8081/](http://localhost:8081/) で確認できます。生成先は `dist/` です。

## Markdown

コードフェンスに `rust`、`javascript`、`python`、`bash` などの言語名を指定すると、
サーバー表示と静的出力で構文ハイライトが適用されます。
言語指定なし・未対応言語・インデント形式は通常のテキスト表示です。
長い行は横スクロールできます。

## 本番コンテナ

```bash
docker build -f prod/Dockerfile -t rust-blog:prod .
docker run --rm -p 8080:8080 -v "$(pwd)/data:/data" rust-blog:prod
```

起動時に SQLite DB の準備、マイグレーション、サーバー起動を行います。
seed は事前に実行してください。

- `PORT`: 待受ポート（既定: `8080`）
- `DB_PATH`: DB ファイル（既定: `/data/blog.db`）
- `DATABASE_URL`: DB 接続先を直接指定する場合に使用

## 関連ドキュメント

- [テスト観点](docs/testing.md)
- [目次設定](docs/table-of-contents.md)
- [静的デプロイ](docs/static-deploy.md)
- [export バンドル](docs/export-bundle-usage.md)
- [Cloudflare](docs/cloudflare.md)
- [出力アーキテクチャ](docs/dual-output-architecture.md)
