## 開発ルール

## ドキュメントについて
- 実装と矛盾するドキュメントがあれば更新または削除する
    - 対象のドキュメントはマークダウンとする
    - 冗長な表現についてもなくす
    - いずれかのドキュメントが更新された場合はすべてドキュメントを参照し必要に応じて修正する
- ソースコード上のコメントは基本不要

## ブランチ運用

- 作業開始時に、現在のブランチと未コミットの変更を確認する。
- 新しい作業ブランチを作成する前に origin の最新情報を取得する。
- 作業ブランチは原則として最新の origin/develop から、feature/issue-<番号> の形式で作成する。
- 既存の作業を再開する場合は、その作業ブランチを使用する。
- 他の未マージの作業に依存する場合は、依存先の作業ブランチから派生させる。
- 依存する作業の PR は、依存先の作業ブランチをベースとするstacked PR として作成し、本文に依存先 PR を記載する。
- 依存先 PR のマージ後は、後続 PR のベースを develop に変更し、差分に意図した変更だけが含まれることを確認する。
- 既存の未コミット変更を勝手に破棄しない。

## Rust の設計方針

- エラーは呼び出し側へ伝播するか、その場で適切に処理する。
    - 意図的に無視する場合は、その理由をコメントに記載する。
- 回復可能な失敗は Result で返す。
- 呼び出し側が分岐する必要のあるエラーは、型で区別できるようにする。
- 不要な clone() を避け、借用で自然に表現できる場合は借用を使う。
    - ただし、clone() の削減だけを目的に所有権やライフタイムを複雑にしない。
- テストを追加する場合は既存のテストと役割が被っていないかを確認し追加または削除をすること

## 開発環境
- 基本 Docker を使用すること
- Docker container の機能が不足していれば install するか、install しても不足している場合は作業を中断しその内容を書くこと
- Docker まわりの修正があればそれも併せてコミットすること
- このリポジトリでは、Codex による確認・テスト・実行は原則 Docker 経由で行う。

## Issue をまとめて渡されたときの実装方針
1. 渡された Issue の順番で実装をする
2. 実装後はブランチを publish し、PR を作成する
    - この時のブランチ運用はブランチ運用の説明に従う
3. 渡された最後の Issue の PR を作成し終えたら、今回の作業で作成した各 PR の新規の指摘を一巡だけ確認する
    - 妥当な指摘があれば修正・検証し、push 後に該当 PR 内で `@codex review` とコメントする
    - 新規の指摘がない場合、または確認した指摘に修正が不要な場合は、追加のレビュー依頼を行わない
4. 一巡分の対応が終わったらレビュー確認を終了し、実装結果・PR・検証結果を報告する
    - レビューが未完了または再レビュー待ちの場合は、その状態も報告する
    - 新しい指摘を待つための自動ポーリングや、無条件の繰り返しは行わない。後続のレビュー対応はユーザーからの依頼時に再開する

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
  - `docker compose up -d --force-recreate static`
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
- `docker run --rm -v "${PWD}/data:/data" -v "${PWD}/artifact:/export" rust-blog-prod-local export /export/dist`

export はディレクトリを rename して切り替えるため、出力先そのものではなく親ディレクトリをマウントする。この例のホスト側出力先は `artifact/dist/`。

## Editing Notes

- テンプレートや静的アセットを変更したら、必要に応じて `export` を再実行して `static` で確認する。
- サーバーモードと静的出力モードで URL 形式が異なる箇所があるため、片方だけで確認して完了にしない。
- `dist/` は生成物として扱う。必要なときだけ再生成する。
- export 成功後は `static` を `--force-recreate` で再作成する。出力ディレクトリを置き換えるため、既存コンテナの bind mount は古いディレクトリを参照し続ける。
