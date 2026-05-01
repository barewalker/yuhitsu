#set text(lang: "ja", font: "Harano Aji Mincho", size: 10.5pt)
#set page(paper: "{{paper}}", margin: 2.5cm, numbering: "1")
#set par(justify: true, leading: 0.85em, first-line-indent: 1em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(size: 14pt)
#show heading.where(level: 2): set text(size: 12pt)

#align(center)[
  #text(size: 18pt, weight: "bold")[技術報告書のタイトル]
  #v(0.5em)
  著者名 / 所属
  #v(0.3em)
  #datetime.today().display()
]

#v(1em)

#align(center)[
  #box(width: 90%)[
    #set par(first-line-indent: 0pt)
    *概要* — ここに 200 字程度の要旨を書く。研究背景、目的、手法、得られた主要な結果を簡潔に述べる。
  ]
]

#v(1em)

= はじめに
研究背景・先行研究・目的を記述。

= 方法
実施した手法・条件・装置。再現可能なレベルで詳細に。

= 結果
計測値・図表。

// 図の挿入例
// #figure(
//   image("figure1.png", width: 80%),
//   caption: [図 1: 実験装置の構成],
// )

= 考察
結果の解釈・先行研究との比較・限界。

= 結論
本研究の到達点と今後の展望。

// 参考文献ファイルは Hayagriva YAML (.yml) と BibTeX (.bib) の両方に対応。
// Hayagriva は Typst ネイティブで構造が直感的(推奨)。
// BibTeX は学術界の標準で既存資産が活きる。どちらか一方を選んで利用。
#bibliography("references.yml")
// #bibliography("references.bib")
