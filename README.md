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
   sea-orm-cli generate entity -o entity/src/entity
   ```

2. `entity/src/entity/*.rs` に `Relation` 含むコードが生成されることを確認。

## シード用バイナリの実行

1. `core` クレートをビルドしつつ seed を実行：

   ```bash
   cargo run -p rust_blog --bin seed
   ```

   - `seed.rs` が `content/articles/*.md` を読み込み DB に投入します。

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
