# テスト記事の一括 export と Cloudflare 公開

`content/articles/*.md` の全記事と `content/fixed_contents/*.md` の固定ページを、
一時 SQLite DB に取り込んで `dist/` へ静的出力します。
既存の `blog.db` は変更しません。`dist/` は実行ごとに再生成されます。
`deleted: true` の記事は公開対象に含まれません。

## 記事の作成日時

静的 export では記事の Front Matter に `created_at`（別名 `date`）が必須です。
未指定の記事があると export を停止します。毎回一時 DB を作るため、
日付を省略すると seed の実行日時が CREATE DATE になってしまうのを防ぎます。

```yaml
created_at: "2026-09-20T10:00:00+09:00"
```

日付のみの `"2026-09-20"` は日本時間の午前0時として扱います。
既存の日付未指定記事には Git 履歴の初回登録日時を補いました。
実際の公開日時と異なる場合は `created_at` を修正してください。
通常のサーバー向け seed は従来どおり、日付省略時に既存 DB の作成日時を維持します。

## ローカルで生成・確認

```sh
docker compose up -d web
docker compose exec -T web sh docker/export-articles.sh
docker compose up -d --force-recreate static
```

`http://localhost:8081/` を開いて一覧・記事・画像を確認します。
記事を追加・編集したときも同じ export コマンドを実行します。
export は出力ディレクトリを作り直すため、実行後は上記のコマンドで
`static` コンテナも再作成してマウントを更新します。

## GitHub Actions で生成

入力パスと出力先は `.github/workflows/deploy_static.yml` の `env` で設定します。
これらを `docker compose exec -e` でコンテナへ渡します。
`docker/export-articles.sh` はリポジトリ直下から実行し、環境変数を読み込みます。
ローカルで未指定の場合は既存の標準配置を使います。

サーバーと export の設定ファイルは `RUST_BLOG_CONFIG_PATH` で指定できます。
サーバーの既定値は `blog_config.toml` です。一括 export では seed にも同じ設定を渡します。
`[common]` の `image_dir` / `icon_dir` はサーバーの配信元と export のコピー元に共通です。
指定時は `RUST_BLOG_CONTENT_DIR` より優先し、相対パスは作業ディレクトリ基準です。
未指定時の export は従来どおりコンテンツディレクトリ配下の `image` / `icon` を使います。

変更を GitHub のデフォルトブランチへ反映した後、Actions の
**Export articles and deploy** → **Run workflow** を実行します。
`deploy` がオフなら Cloudflare の認証なしで export だけ実行できます。
生成した HTML・画像などは `static-site` artifact としてダウンロードできます。

## Cloudflare へデプロイ

1. Cloudflare アカウントを用意し、Account ID を確認します。
2. API Tokens で **Edit Cloudflare Workers** テンプレートからトークンを作成し、
   対象アカウントに範囲を限定します。
3. GitHub リポジトリの Settings → Secrets and variables → Actions に
   `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` を repository secrets として登録します。
   トークンをファイルへ書き込む必要はありません。
4. **Run workflow** で `deploy` をオンにして実行します。

`wrangler.toml` の `rust-blog-static` という名前で Workers Static Assets に公開します。
同じ名前の Worker がある場合は、そのデプロイが更新されます。
公開 URL はデプロイステップのログで確認できます。
初回に workers.dev サブドメインの設定を求められる場合は、Cloudflare ダッシュボードで設定します。

公開先では Rust サーバーや SQLite を動かさず、生成済みファイルを配信します。
この構成に D1 や Containers の契約は不要です。
記事更新時は workflow を再実行して生成・公開します。

公式手順: https://developers.cloudflare.com/workers/ci-cd/external-cicd/github-actions/
