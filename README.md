**Rust Blog (Rocket + SeaORM) Workspace Setup & Operations**

この README では、ワークスペース構成のもとで SeaORM のマイグレーション／エンティティ生成／シード／サーバ起動までの一連の操作手順をまとめています。

---

## 目次

1. [前提条件](#前提条件)
2. [リポジトリ構成](#リポジトリ構成)
3. [環境変数](#環境変数)
4. [マイグレーションの適用](#マイグレーションの適用)
5. [エンティティの生成](#エンティティの生成)
6. [シード用バイナリの実行](#シード用バイナリの実行)
7. [アプリケーションの起動](#アプリケーションの起動)
8. [Docker 開発環境](#docker-開発環境)
9. [トラブルシューティング](#トラブルシューティング)
10. [テスト観点まとめ](#テスト観点まとめ)

---

## 前提条件

- Rust (rustup + cargo) がインストール済み
- Docker インストール済み

## リポジトリ構成

```
rust_blog/             ← ワークスペースルート
├── Cargo.toml         ← ワークスペース定義
├── blog.db            ← SQLite DB ファイル
├── core/              ← アプリケーションクレート
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── bin/seed.rs
└── entity/            ← SeaORM エンティティクレート
    ├── Cargo.toml
    └── src/
```

## マイグレーションの適用

1. ワークスペースルートでマイグレーションを実行：

   ```bash
   sea-orm-cli migrate up
   ```

2. テーブル構造が `blog.db` に作成されることを確認します。

## エンティティの生成

1. マイグレーション適用後、`entity` クレートにモデルを出力：

   ```bash
   sea-orm-cli generate entity -o core/src/entity
   ```

2. `core/src/entity/*.rs` に `Relation` 含むコードが生成されることを確認。

## シード用バイナリの実行

1. `core` クレートをビルドしつつ seed を実行：

   ```bash
   cargo run -p rust_blog --bin seed
   ```

   - `seed.rs` が `content/articles/*.md` を読み込み DB に投入します。

### Markdown のコードブロック

コードフェンスの直後に `rust`、`javascript`、`python`、`bash` などの言語名を指定すると、構文ハイライトが適用されます。

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

- ハイライトは HTML 生成時に行うため、JavaScript や外部 CDN は不要です。
- サーバー表示と静的 export の両方で適用されます。既存の静的サイトには export の再実行が必要です。
- 言語指定なし・未対応言語・インデント形式のコードブロックは、通常のテキスト表示になります。
- 長い行は折り返さず、コードブロック内で横スクロールできます。

## アプリケーションの起動

1. rust_blog を起動：

   ```bash
   cargo run -p rust_blog
   ```

2. 以下にアクセス:

   - 記事一覧: [http://localhost:8888/](http://localhost:8000/)
   - 記事詳細: [http://localhost:8888/posts/](http://localhost:8000/posts/<slug>)
   - タグ一覧: [http://localhost:8888/tags](http://localhost:8000/tags)

## Docker 開発環境

1. Docker イメージのビルド＆起動：

   ```bash
   docker-compose up --build -d
   ```

## トラブルシューティング

- **外部キー制約を生成しているのに Relation が生成されない**

  - `migrate up` → `generate entity` を順に確実に実行する
  - `--verbose` フラグで FK 検出ログを確認

- **SQLite ファイルがディレクトリになる**

  - ホストに `touch blog.db` で空ファイルを作成

- **バージョン衝突**

  - workspace 内で対象のライブラリのバージョンを揃える

## テスト観点まとめ

テストで何を検証しているかは `docs/testing.md` にまとめています。

## デプロイメモ

テスト記事をまとめて静的 export し、GitHub Actions から Cloudflare に公開する手順は
[静的デプロイ手順](docs/static-deploy.md) を参照してください。

Cloudflare へ載せるときの考え方と選択肢は `docs/cloudflare.md` にまとめています。

### コンテナで最短公開する場合

Cloudflare を DNS / CDN / WAF として使い、アプリ本体は別のコンテナ基盤で動かす構成を取りやすいように、`prod/Dockerfile` を用意しています。

ビルド:

```bash
docker build -f prod/Dockerfile -t rust-blog:prod .
```

起動:

```bash
docker run --rm \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  -e PORT=8080 \
  rust-blog:prod
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
