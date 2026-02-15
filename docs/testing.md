# Testing Guide

このドキュメントは、このリポジトリで「何をテストで検証しているか」を整理したものです。

## 実行方法

ワークスペースルートで実行:

```bash
cargo test
```

## テスト観点一覧

### ページング (`core/src/domain/page.rs`)

- `Page::new_from_query`
  - クエリ未指定時のデフォルト値 (`count=1`, `per=10`)
  - クエリ指定時の値反映
- `Page::normalize`
  - `count` の最小値補正
  - `per` の下限・上限クランプ
- `PageInfo::new`
  - `total_pages` の算出
  - `has_prev` / `has_next` の境界判定
  - `prev_page` / `next_page` の境界値
- `PageInfo::get_prev_url` / `get_next_url`
  - ページ遷移不可時に空文字を返す
  - `sort_key` 付き URL 生成

### スラッグ設定 (`core/src/slug_config.rs`)

- TOML から `map` を正しく読み取れること
- 指定テーブルキーのみを抽出できること
- 存在しないテーブルキー指定時にエラーになること

### 共通設定 (`core/src/utils/config.rs`)

- TOML から `CommonConfigMap` を読み取れること
- 指定テーブルキーの抽出ができること
- 存在しないテーブルキーでエラーになること
- `load_config` 相当の処理で以下が保証されること
  - 必須キー欠落時のデフォルト補完
  - 設定ファイルが無い場合のフォールバック

### Seed パス設定 (`core/src/seed/config.rs`)

- `PathConfig::new` のデフォルト値
- `PathConfig::new` の指定値反映
- `PathConfig::update` が全フィールドを更新すること

### Markdown Seed 解析 (`core/src/seed/markdown.rs`)

- `markdown_files`
  - 再帰的に `.md` ファイルのみ収集すること
- `parse_markdown_to_front_matter`
  - Front Matter と本文の分離
  - YAML 不正時のエラー
  - 区切り (`---`) 不足時の panic
- `parse_markdown_to_fixed_content_matter`
  - Front Matter と本文の分離

### ルートハンドラ (`core/src/route/get/tag.rs`, `core/src/route/get/category.rs`)

- 詳細ページ (`/tag/<slug>`, `/category/<slug>`)
  - `sort_key` 未指定時に `created_at` を使うこと
  - `excerpt` が無い記事で本文から抜粋が生成されること
  - 対象が存在しない時に `404` を返すこと
  - 想定外 DB エラー時に `500` を返すこと

## 今後の拡張候補

- `index` / `post_detail` の HTTP レスポンス観点追加
- `repository/article.rs` の並び順分岐 (`created_at` / `updated_at`) の直接検証
- `seed/article/seed.rs` の `prepare` / `upsert` 分岐を追加
