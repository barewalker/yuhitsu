#set text(lang: "ja", font: "Harano Aji Mincho", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.85em)
#set heading(numbering: "1.")

#align(center)[
  #text(size: 16pt, weight: "bold")[業務報告書]
]

#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 6pt,
  [作成者], [],
  [所属], [],
  [対象期間], [#datetime.today().display("[year]/[month]/[day]") 〜],
  [作成日], [#datetime.today().display()],
)

= 実施内容
// ここに実施内容を箇条書きで記載
-

= 進捗 / 成果

= 課題 / 所感

= 次回予定
