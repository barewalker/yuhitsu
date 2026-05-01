// 用紙サイズ設定 ({{paper}}) は本テンプレでは未使用。スライドは 16:9 固定。
// 将来 touying / polylux 統合時に再設計予定。
#set text(lang: "ja", font: "Harano Aji Gothic", size: 22pt)
#set page(paper: "presentation-16-9", margin: 2cm)
#set par(leading: 0.9em)

// タイトルスライド
#align(center + horizon)[
  #text(size: 40pt, weight: "bold")[プレゼンテーションのタイトル]
  #v(1em)
  #text(size: 24pt)[サブタイトル]
  #v(2em)
  #text(size: 18pt)[発表者名]
  #v(0.5em)
  #text(size: 16pt)[#datetime.today().display()]
]

#pagebreak()

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
