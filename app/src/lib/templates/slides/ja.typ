// 用紙サイズ設定 ({{paper}}) は本テンプレでは未使用。スライドは 16:9 固定。
// 将来 touying / polylux 統合時に再設計予定。
#set text(lang: "ja", font: "Harano Aji Gothic", size: 22pt)
#set page(paper: "presentation-16-9", margin: 2cm)
#set par(leading: 0.9em)

#let slides(
  title: "プレゼンテーションのタイトル",
  subtitle: "",
  presenter: "",
  body,
) = {
  set document(title: title, author: presenter)
  align(center + horizon)[
    #text(size: 40pt, weight: "bold")[#title]
    #v(1em)
    #if subtitle != "" [
      #text(size: 24pt)[#subtitle]
      #v(2em)
    ]
    #text(size: 18pt)[#presenter]
    #v(0.5em)
    #text(size: 16pt)[#datetime.today().display()]
  ]

  pagebreak()

  body
}

#show: slides.with(
  title: "プレゼンテーションのタイトル",
  subtitle: "サブタイトル",
  presenter: "発表者名",
)

// 目次
= 目次
+ はじめに
+ 本論
+ まとめ

#pagebreak()

= はじめに
- 背景
- 目的
- 課題

#pagebreak()

= 本論
- ポイント 1
- ポイント 2
- ポイント 3

#pagebreak()

= まとめ
- 結論
- 今後の展望

#pagebreak()

#align(center + horizon)[
  #text(size: 48pt, weight: "bold")[ご清聴ありがとうございました]
]
