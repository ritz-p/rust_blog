# Article metadata

Set `public_url = "https://blog.example.com"` in `[common]` of `blog_config.toml` to enable absolute canonical and Open Graph URLs. Use the public site root without a query or fragment.

Article pages include a plain-text description (excerpt, or the first 160 characters of the body), Open Graph title, description, site name and type. The article image falls back to `default_icatch_path`; relative image paths use the public URL. Without a public URL, canonical/og:url and relative og:image are omitted.

Server URLs use `/posts/slug`; static URLs use `/posts/slug/`. Slugs are URL encoded and metadata is HTML escaped. Re-export after changing configuration.

## ブログトップの共有

```toml
[common]
site_name = "Ritz のブログ"
description = "Rust と Web 開発についてのブログ"
public_url = "https://blog.example.com"
default_icatch_path = "/image/cover.png"
```

トップは `og:type=website` とサイト共通の画像を使います。記事は `og:type=article` と記事ごとの画像を使い、省略時はサイト共通の画像を使います。画像 URL を出力するときは X 向けに `twitter:card=summary_large_image` も出力します。相対画像 URL は `public_url` から絶対 URL に解決します。

## 記事本文のリンクカード

記事の段落に URL を1つだけ書くと、リンク先のタイトルとドメインが表示されます。箇条書きでも同じ書き方が使えます。

```markdown
- https://planorama.dev/
```

リンク先に Open Graph の説明文や HTTPS のサムネイルがあればカードに表示します。タイトルと説明文には HTML の `title` と `meta name="description"` も使えます。取得できない場合は元の URL をクリックできます。文中のリンクやコードブロックは通常どおり表示されます。

サーバーは記事表示時、静的サイトは export 時に公開 HTTP(S) ページを取得します。同じ URL の取得結果はプロセス内で15分間再利用されます。静的サイトのカードを更新するには再度 export してください。
