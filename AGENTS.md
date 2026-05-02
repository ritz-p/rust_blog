# Codex Instructions

このリポジトリでは、Codex による確認・テスト・実行は原則 Docker 経由で行う。

## Execution Policy

- ホスト環境で直接 `cargo test` や `cargo run` を実行するのは、Docker で再現できない場合だけにする。
- まず `docker compose` を使った開発コンテナを優先する。
- Rust のテスト、ビルド、`cargo run`、`sea-orm-cli` は `web` サービス内で実行する。
- 静的出力の確認は `dist/` 直開きではなく `static` サービス経由で行う。

## Preferred Commands

- 開発コンテナ起動:
  - `docker compose up -d web`
- テスト:
  - `docker compose run --rm web cargo test -p rust_blog`
  - もしくは `docker compose exec web cargo test -p rust_blog`
- 開発サーバー起動:
  - `docker compose exec web cargo run -p rust_blog`
- seed:
  - `docker compose exec web cargo run -p rust_blog --bin seed`
- 静的 export:
  - `docker compose exec web cargo run -p rust_blog --bin export`
- 静的 preview:
  - `docker compose up -d static`
  - `http://localhost:8081/` で確認

## Production Container Checks

- 本番イメージの確認が必要な場合だけ `prod/Dockerfile` を使う。
- `prod/entrypoint.sh` のサブコマンドを使う。
  - `server`
  - `export`
  - `migration`

例:

- `docker build -f prod/Dockerfile -t rust-blog-prod-local .`
- `docker run --rm -p 8080:8080 -v "${PWD}/data:/data" rust-blog-prod-local`
- `docker run --rm -v "${PWD}/data:/data" -v "${PWD}/dist:/app/dist" rust-blog-prod-local export`

## Editing Notes

- テンプレートや静的アセットを変更したら、必要に応じて `export` を再実行して `static` で確認する。
- サーバーモードと静的出力モードで URL 形式が異なる箇所があるため、片方だけで確認して完了にしない。
- `dist/` は生成物として扱う。必要なときだけ再生成する。
