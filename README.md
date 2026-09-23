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
seed は読み込み・解析・検証・DB 操作のエラーをファイル単位で集約し、処理可能なファイルを最後まで処理します。
最後にエラー一覧を表示し、1 件でも失敗した場合は非ゼロで終了します。成功した変更は DB に残ります。DB に接続できない場合は処理を開始できません。

エンティティの再生成:

```bash
docker compose exec web sea-orm-cli generate entity -o core/src/entity --with-serde both
```

テスト:

```bash
docker compose exec web cargo test -p rust_blog
```

Markdown・JavaScript の CI は master 向け PR と master への push で実行します。
ワークフローは `markdown_checks.yml` と `javascript_checks.yml` に分かれており、それぞれ独立して実行します。静的 export・配信確認は JavaScript 側で実行します。
Markdown のリンク・見出しなどの lint、記事 front matter の検証・整形チェック、JavaScript の lint、検索とナビゲーションの DOM テストを行います。
静的 export は一時 DB から生成し、nginx 経由で HTML・JavaScript・検索インデックス・記事の配信も確認します。

```bash
docker compose exec web npm ci
docker compose exec web npm run check
docker compose exec web cargo run -p rust_blog --bin format_markdown -- --check content/articles
docker compose exec web sh docker/export-articles.sh
docker compose up -d --force-recreate static
docker compose exec -e STATIC_SITE_URL=http://static web npm run test:js
```

`npm run check` のみでは静的配信テストをスキップします。配信も確認する場合は上記の export・static 起動後に `STATIC_SITE_URL` を指定してください。

## 静的出力

```bash
docker compose exec web cargo run -p rust_blog --bin export
docker compose up -d --force-recreate static
```

[http://localhost:8081/](http://localhost:8081/) で確認できます。生成先は `dist/` です。

## 記事検索

記事一覧の検索欄から、タイトル・概要・本文を部分一致で検索できます。
英字の大文字・小文字は区別せず、空白で区切った複数語はすべて含む記事を表示します。
検索語は URL の `q` に保存され、ページ送りと月別絞り込みでも保持されます。
「解除」で通常の一覧へ戻れます。

サーバーモードは公開済み記事を読み込み、Markdown をテキスト化して検索します。
静的サイトは export 時に生成する `search-index.json` をブラウザーで検索するため、
JavaScript が必要です。記事の変更を検索へ反映するには export を再実行してください。

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
このイメージは起動時に以下を行います。
1. SQLite ファイルを `/data/blog.db` に用意
2. migration を適用
3. `rust_blog` を起動

`seed` は実行時には行いません。記事投入や初期データ作成はデプロイ前に済ませておく前提です。

主な環境変数:

- `PORT`: アプリ待受ポート。デフォルトは `8080`
- `DB_PATH`: SQLite ファイルの配置先。デフォルトは `/data/blog.db`
- `DATABASE_URL`: 明示指定したい場合に使用

Cloudflare 側では、このコンテナを配置したホストへ DNS を向けて Proxy を有効化します。

## SeaOrm について

1. テーブル作成

```bash
seaorm migrate generate ${table_name}
```

2. 生成されたファイルを編集してテーブル定義を書く

# migration/src/mYYYYMMDDHHMMSS\_${table_name}.rs

3. マイグレーションを適用

```bash
seaorm migrate up -u "$DATABASE_URL"
```

4. Entity を DB から再生成

```bash
seaorm generate entity -u "$DATABASE_URL" -o core/src/entity --with-serde both
```

5. 適用状況を確認

```bash
seaorm migrate status -u "$DATABASE_URL"
```

6. 全部やり直す（drop→up）

```bash
seaorm migrate refresh -u "$DATABASE_URL"
```

## 記事 Markdown の整形・検証

記事・固定ページの slug は 1〜100 UTF-16 コード単位かつ UTF-8 で 255 バイト以内です。空白のみ・前後の空白、パス区切り、制御文字、Windows で使えないファイル名を拒否します。日本語や内部の空白は使用できます。予約名の判定では拡張子直前の空白も除き、`CON .txt`・`LPT1 .md` なども拒否します。
seed は入力全体の slug を事前検証し、記事同士・固定ページ同士の重複（大文字小文字を区別しない）をファイル名つきで報告します。記事と固定ページは別の名前空間です。
slug が不正なファイルと重複に関わる全ファイルは保存せず、その他のファイルを処理してからエラーをまとめて表示します。
重複判定には Unicode case folding を使い、`Σ` と `ς`、`Straße` と `STRASSE` も同一キーとして扱います。保存する slug 自体は変更しません。Windows の予約名には `COM¹`・`LPT²` などの上付き数字や拡張子つきの名前も含みます。
比較キーは case folding の前後で NFD 正規化し、`é` と `e`＋結合アクセントのような正準等価な表記も重複として扱います。固定ページの `post` は使用できますが、記事の出力先である `posts` は予約名です。
Windows での衝突も防ぐため、比較キーには大文字変換も適用し、`I`・`i`・`ı` を同一扱いにします。各 OS のファイルシステムと完全に同じ比較ではなく、移植性のために保守的に重複を拒否します。`CONIN$`・`CONOUT$` も予約名です。
既存 DB の同じ slug は通常どおり更新します。固定ページでは `posts`・`tags`・`js` などの既存ルートや出力ファイル名も使えません。
整形 CLI も記事 slug の形式・入力内の重複を検証します。静的 export は DB 内の記事・固定ページの slug を出力先の初期化前に検証します。
整形 CLI は書き込み前に全入力の重複を確認し、重複に関わるファイルはすべて変更せず、その他の正常なファイルを整形します。

```bash
docker compose exec web cargo run -p rust_blog --bin format_markdown -- content/articles
docker compose exec web cargo run -p rust_blog --bin format_markdown -- --check content/articles
```

省略時の対象は `content/articles`。ファイル・ディレクトリを複数指定できます。
明示指定したシンボリックリンクはリンク先を処理します。ディレクトリ配下のリンクはたどりません。
UTF-8 BOM 付きファイルにも対応し、BOM と改行形式を保持します。
必須項目は `title`、`slug`、`tags`、`categories`（配列は空でも可）。
型・文字数・日時・未知の項目を検証します。不正なファイルは書き換えず、正常なファイルの整形を続けます。
探索・読み書き・検証のエラーは最後にファイル名つきでまとめて表示し、非ゼロで終了します。
`--check` では書き換えず、検証エラーと未整形のファイルをまとめて報告します。
順序は `title`, `slug`, `deleted`, `table_of_contents`, `created_at`, `excerpt`, `icatch_path`, `tags`, `categories`。
`date` は名前を保持し、`created_at` と同じ位置に並べます。省略された任意項目は追加しません。
YAML のコメントや引用形式は正規化されますが、本文は保持します。
`--check` は検証エラーまたは未整形の場合に非ゼロで終了します。
## DB から Markdown を出力

```bash
docker compose exec web cargo run -p rust_blog --bin export_markdown
```

`DATABASE_URL` の DB から `markdown_output/articles/<id>.md` と
`markdown_output/fixed_contents/<id>.md` に出力します。引数で出力先を変更できます。
既存ファイルや古い出力があればエラーになります。更新する場合は `--force` を指定してください。
`--force` は上書きに加え、DB に存在しない `articles/<id>.md`・`fixed_contents/<id>.md` を削除します。空の DB でも古い出力を削除します。
削除対象は各ディレクトリ直下の整数 ID のファイルだけです。それ以外のファイルは保持するため、seed 用の出力先には手書きの Markdown を混在させないでください。
記事の公開日時・目次設定・タグ・カテゴリ・本文を保持します。
DB の ID・更新日時および固定ページの日時は seed の入力項目ではないためヘッダーには含めません。
タグ・カテゴリは seed と同じ名前の配列です。独立した taxonomy の slug や未使用項目のバックアップには使えません。

再投入:

```bash
docker compose exec -e ARTICLE_PATH=markdown_output/articles -e FIXED_CONTENT_PATH=markdown_output/fixed_contents web cargo run -p rust_blog --bin seed
```
## 関連ドキュメント

- [テスト観点](docs/testing.md)
- [目次設定](docs/table-of-contents.md)
- [静的デプロイ](docs/static-deploy.md)
- [export バンドル](docs/export-bundle-usage.md)
- [Cloudflare](docs/cloudflare.md)
- [出力アーキテクチャ](docs/dual-output-architecture.md)
