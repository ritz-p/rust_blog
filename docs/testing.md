# Testing Guide

主な既存テストの観点をまとめています。テスト追加前に該当 Rust モジュール内のテストと `tests/frontend/` を確認し、既存の検証と重複しないようにします。

## 実行方法

ワークスペースルートで実行:

```bash
docker compose run --rm web cargo test --workspace
```

develop 向け PR の Cargo Test は、Rust ソース、Cargo manifest / lockfile、`.env`、
Rust・ブログ設定、テスト workflow に加え、`ci/Dockerfile`、`docker-compose.yml`、
`docker-compose.ci.yml`、`.dockerignore`、`.github/actions/prepare-checks/action.yml`
の変更時に実行します。CSS・JavaScript・テンプレート・記事・画像・ドキュメントのみの変更では実行しません。
Markdown・JavaScript・静的配信の検証は [CI の構成](ci-checks.md) と [開発手順](../README.md#開発) を参照してください。

## テスト観点一覧

### ページング (`core/src/domain/page.rs`)

- `Page::new_from_query`
  - クエリ未指定時のページ番号 `1` と、呼び出し側から渡したページサイズ
  - クエリ指定時の値反映
- `Page::normalize`
  - `number` の最小値補正
  - `per` の下限・上限クランプ
- `PageInfo::new`
  - `total_pages` の算出
  - `has_prev` / `has_next` の境界判定
  - `prev_page` / `next_page` の境界値
- `PageInfo::get_prev_url` / `get_next_url`
  - ページ遷移不可時に空文字を返す
  - `sort_key` 付き URL 生成
- `PageInfo::navigation` / `get_page_url`
  - 現在ページの前後3ページと端のページへのリンク
  - ページサイズ・並び順を保持した URL 生成

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
  - 区切り (`---`) 不足時にエラーを返すこと
  - `date` エイリアスと日付省略時の解析
- `parse_markdown_to_fixed_content_matter`
  - Front Matter と本文の分離
- 記事・固定ページの UTF-8 BOM と LF / CRLF の処理

### ルートハンドラ (`core/src/route/get/`)

- 詳細ページ (`/tag/<slug>`, `/category/<slug>`)
  - `sort_key` 未指定時に `created_at` を使うこと
  - `excerpt` が無い記事で本文から抜粋が生成されること
  - 対象が存在しない時に `404` を返すこと
  - 想定外 DB エラー時に `500` を返すこと
- 記事一覧・アーカイブ (`index.rs`)
  - 本文検索、月別絞り込み、JST の月境界
  - 設定したページサイズとクエリによる上書き
  - 検索語・対象月を保持したページング URL
- 記事詳細 (`article.rs`)
  - ハイライト済みコードと対応する CSS の配信
  - deprecated の注意表示と本文より前の配置、未指定記事での非表示

### Seed (`core/src/seed.rs`, `core/src/seed/`)

- 記事の新規登録・更新・削除、Front Matter の検証
- 公開日時の指定・省略・JST と UTC の解釈
- タグ・カテゴリの同期と記事単位のロールバック
- ファイル単位のエラー集約と後続ファイルの処理
- slug の事前検証と、Git 履歴に基づく更新日時

### 静的出力 (`core/src/static_site.rs`, `core/src/static_site/output.rs`)

- 公開日時による記事の除外、検索インデックスと静的 URL
- テンプレート・CSS・JavaScript・設定した画像ディレクトリの反映
- 不正な slug による出力開始前の失敗
- 生成・切り替え失敗時の旧出力保持と、成功時の古いファイル削除

静的 HTML の配信と内部リンクは [リンク検証](link-checks.md) も参照してください。
