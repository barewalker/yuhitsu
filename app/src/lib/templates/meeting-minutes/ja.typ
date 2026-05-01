#set text(lang: "ja", font: "Harano Aji Mincho", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.85em)
#set heading(numbering: "1.")

#align(center)[
  #text(size: 16pt, weight: "bold")[議事録]
]

#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 6pt,
  [日時], [#datetime.today().display() 〜],
  [場所], [],
  [出席者], [],
  [議事録作成], [],
)

= 議題
+

= 議論内容

= 決定事項
-

= 宿題 / 次回までの TODO
#table(
  columns: (1fr, auto, auto),
  stroke: 0.5pt,
  inset: 6pt,
  [内容], [担当], [期限],
  [], [], [],
)
