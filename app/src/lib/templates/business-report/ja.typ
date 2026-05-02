#set text(lang: "ja", font: "Harano Aji Mincho", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.85em)
#set heading(numbering: "1.")

// テンプレ関数定義。フォーム入力欄はこの引数を編集する。
// 引数を増やすと右ペインのフォームに自動で欄が増える(同梱テンプレでは
// meta.json の form.fields がラベル翻訳・型を上書きする)。
#let business-report(
  title: "業務報告書",
  author: "",
  affiliation: "",
  period: "",
  body,
) = {
  set document(title: title, author: author)
  align(center)[
    #text(size: 16pt, weight: "bold")[#title]
  ]

  v(1em)

  table(
    columns: (auto, 1fr),
    stroke: 0.5pt,
    inset: 6pt,
    [作成者], [#author],
    [所属], [#affiliation],
    [対象期間], [#period],
    [作成日], [#datetime.today().display()],
  )

  body
}

#show: business-report.with(
  title: "業務報告書",
  author: "",
  affiliation: "",
  period: "",
)

= 実施内容
// ここに実施内容を箇条書きで記載
-

= 進捗 / 成果

= 課題 / 所感

= 次回予定
