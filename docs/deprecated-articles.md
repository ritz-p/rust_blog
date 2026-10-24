# 古い記事の注意表示

記事 Markdown の YAML フロントマターに `deprecated: true` を指定すると、本文と目次の前に「この記事の内容は古いです」をオレンジ色の notice で表示します。目次の `table_of_contents` と同じく記事ごとの設定で、省略時または `false` の場合は表示しません。

```yaml
---
title: 古い記事
slug: old-article
deprecated: true
tags: []
categories: []
---
```

既存 DB には最初に `docker compose exec web cargo run -p migration -- up` を実行してください。既存記事は `false` になります。

設定を変更したら seed を再実行します。静的サイトでは export 後に `static` を `--force-recreate` で再作成してください。記事の公開・非公開や本文は変更しません。設定を削除するか `false` に戻して seed すると注意表示を解除できます。

Markdown 整形 CLI は真偽値を検証し、DB からの Markdown 書き出しでも設定を保持します。確認用の記事は `content/articles/50.md` です。
