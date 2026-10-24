# Cloudflare Deployment Guide

このリポジトリは静的 HTML を生成し、Cloudflare Workers Static Assets へ公開できます。
公開先では生成済みの `dist/` を配信します。Rocket サーバー・SQLite・D1 を Cloudflare 上で動かす必要はありません。

## 現在の構成

| 処理 | 実行場所・実装 |
| --- | --- |
| Markdown を一時 SQLite DB へ投入 | Docker の `web` サービスで migration と seed を実行 |
| HTML・CSS・JavaScript・画像を生成 | `export` で `dist/` へ出力 |
| 生成物を Cloudflare へ配置 | `.github/workflows/deploy_static.yml` の Wrangler ステップ |
| サイトを配信 | `wrangler.toml` で指定した Worker の Static Assets |

CI は静的サイトと DB を別々の artifact に保存します。Cloudflare へ配置するのは静的サイトだけです。
記事を更新した後は export とデプロイを再実行します。未来の公開日時を設定した記事も、その日時以降の再デプロイで公開されます。

## 公開手順

ローカル生成、プレビュー、GitHub Actions の secrets、手動デプロイ、公開先の設定は [静的デプロイ](static-deploy.md) を参照してください。

- Actions の **Export articles and deploy** は、手動実行時の `deploy` がオフなら生成だけを行います。
- 定期公開の有効化と実行時刻は [予約公開](scheduled-publication.md) を参照してください。
- 別の記事リポジトリで生成する場合は [export バンドル](export-bundle-usage.md) を使用できます。
- サイトの canonical URL・sitemap は [メタデータ](article-metadata.md) と [検索エンジン向け出力](search-engine-discovery.md) を参照してください。

## サーバーモードとの関係

`prod/Dockerfile` は Rocket と SQLite を使うサーバーモード用です。永続化した DB を持つホストで動かし、Cloudflare を DNS・Proxy として利用する場合の手順は [README](../README.md#本番コンテナ) にあります。

Workers がリクエストごとに D1 を読み込んで HTML を生成するアプリは実装していません。その方式を選ぶ場合は、ルーティング・DB アクセス・migration・seed の対応を別途設計します。現在の静的公開にその実装は必要ありません。
