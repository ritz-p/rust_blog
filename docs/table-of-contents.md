# 記事の目次

Markdown のフロントマターで `table_of_contents: true` を指定すると、本文の先頭に目次を表示します。未指定または `false` の場合は表示しません。

```yaml
---
title: "目次のある記事"
slug: "example"
created_at: "2026-09-21T00:00:00Z"
table_of_contents: true
tags: []
categories: []
---
```

本文の Markdown 見出し（H1〜H6）が対象です。見出しの階層に応じたリストとページ内リンクを生成します。同名の見出しには別々のリンクが付きます。コードブロック内の `#` は対象外です。

設定を変更したら seed を再実行します。静的サイトでは export も再実行してください。サーバー表示と静的出力の両方で有効です。

既存のデータベースには、seed やサーバー起動前に新しいマイグレーションを適用してください。既存記事の設定は `false` になります。

```bash
docker compose exec web cargo run -p migration -- up
docker compose exec web cargo run -p rust_blog --bin seed
docker compose exec web cargo run -p rust_blog --bin export
```

確認用の記事は `content/articles/31.md` です。
