# Dual Output Architecture

このリポジトリはサーバー表示と静的 HTML 出力に対応しています。

- サーバーモード: Rocket が DB から読み込み、リクエストごとに HTML を返す
- 静的モード: DB から `dist/` を生成し、Pages / Workers Static Assets などへ配置する

## 実装の分担

| 責務 | 主な実装 |
| --- | --- |
| 記事・タグ・カテゴリ・固定ページの取得 | `core/src/repository/` |
| 共通の表示データ・ページング・Markdown 変換 | `core/src/view.rs`、`core/src/domain/`、`core/src/utils/` |
| サーバーのレスポンス生成 | `core/src/route/get/` |
| 静的ファイルの生成と出力先の切り替え | `core/src/static_site.rs`、`core/src/static_site/output.rs` |
| 両モードの HTML テンプレート | `templates/` |

テンプレートと Markdown 描画処理は両モードで共有します。URL やテンプレートへ渡す context の組み立てには、各出力側に残っている処理もあります。context の共通化を進める場合も、以下の URL 形式と配信方法の違いを保持します。

## URL 設計

サーバーモードではページ番号・並び順・検索語などをクエリで受け取ります。月別アーカイブにはパス形式もあります。

- `/`
- `/?page=2&per=10&q=Rust`
- `/archive/2026/04`
- `/archive/2026/04/page/2`
- `/posts/<slug>`
- `/tags`
- `/tag/<slug>?page=2&sort_key=updated_at`
- `/categories`
- `/category/<slug>?page=2&sort_key=updated_at`
- `/<fixed-content-slug>`

静的モードではページング・並び順ごとにファイルを生成します。検索は `q` クエリを JavaScript が読み込み、検索インデックスを絞り込みます。

- `/`
- `/page/2/`
- `/archive/2026/04/`
- `/archive/2026/04/page/2/`
- `/posts/<slug>/`
- `/tags/`
- `/tag/<slug>/`
- `/tag/<slug>/page/2/`
- `/tag/<slug>/updated/`
- `/tag/<slug>/updated/page/2/`
- `/categories/`
- `/category/<slug>/`
- `/category/<slug>/page/2/`
- `/category/<slug>/updated/`
- `/category/<slug>/updated/page/2/`
- `/<fixed-content-slug>/`

## 生成物

`export` は次を出力します。

- 一覧・記事・固定ページ・タグ・カテゴリの HTML と `404.html`
- `css/bulma.min.css`、`css/site.css`
- `js/nav.js`、`js/search.js`、`search-index.json`
- 設定したコピー元に存在する画像・アイコン (`image/`、`icon/`)
- Cloudflare 向けの `_headers`、`_redirects`
- `robots.txt` と、`public_url` 設定時の `sitemap.xml`

出力は一時ディレクトリで生成・検証してから切り替えます。失敗時の保持・復元と Docker のマウント条件は [静的デプロイ](static-deploy.md) を参照してください。

## 実行方法

リポジトリのルートで `web` を起動し、[開発手順](../README.md#開発)に従って migration と seed を済ませます。

```bash
docker compose exec web cargo run -p rust_blog --bin export
docker compose up -d --force-recreate static
```

生成先は `dist/` です。[http://localhost:8082/](http://localhost:8082/) から確認します。再生成後も `static` を再作成してマウントを更新してください。

出力先を変える場合:

```bash
docker compose exec web cargo run -p rust_blog --bin export -- artifact/dist
```

標準の `static` サービスは `dist/` を参照するため、別の出力先をプレビューする場合は bind mount も変更します。Markdown から毎回新しい DB を使って生成する手順は [記事の一括 export](static-deploy.md) を参照してください。
