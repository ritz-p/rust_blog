# Dual Output Architecture

このリポジトリを次の 2 形態で出せるようにするための設計です。

- サーバーモード: Rust サーバーとしてデプロイ
- 静的モード: `dist/` を生成して Pages / Workers Static Assets へ配置

## 方針

HTML を返す責務を `Rocket handler` に閉じ込めず、次の 3 層に分けます。

1. `repository`
2. `render data`
3. `delivery`

### repository

DB から記事、タグ、カテゴリ、固定ページを取得する層です。

### render data

テンプレートへ渡すデータを作る層です。

- article detail
- fixed content
- index
- tag list / tag detail
- category list / category detail

この層では `Rocket` も `Cloudflare` も意識しません。

### delivery

出力先ごとの差分だけを受け持ちます。

- サーバーモード: HTTP Response
- 静的モード: `dist/**/*.html`

## 実装方針

### 現段階

まずは既存サーバーを壊さずに、静的 export を追加します。

- 既存の `Rocket` ルートは維持
- `export` バイナリを追加
- `Tera + DB` で静的 HTML を生成
- 画像 / icon / CSS / JS を `dist/` へ出力

### なぜこれを先にやるか

`Rocket -> axum` や `Workers` 直行より、まず `静的 export が成立する形` を作る方が再利用境界が見えやすいからです。

## URL 設計

サーバーモードでは現行 URL を維持します。

- `/`
- `/posts/<slug>`
- `/tags`
- `/tag/<slug>?page=2&sort_key=updated_at`
- `/categories`
- `/category/<slug>?page=2&sort_key=updated_at`
- `/<fixed-content-slug>`

静的モードでは query ベースのページングをそのままファイルへ落とせないため、静的向けの canonical path を持ちます。

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

`export` 実行後の `dist/` には次を出します。

- HTML
- `css/bulma.min.css`
- `css/site.css`
- `js/nav.js`
- `image/*`
- `icon/*`

## 次の段階

この export が安定したら、次を進めます。

1. route 層から context 組み立てをさらに共通化
2. server 実装を `axum` へ寄せるか判断
3. Cloudflare 用に `dist/` 配信か Worker 併用へ寄せる

## 実行方法

前提:

- `DATABASE_URL` が設定されている
- migration / seed 済み

実行:

```bash
cargo run -p rust_blog --bin export
```

出力先を変える場合:

```bash
cargo run -p rust_blog --bin export -- dist
```
