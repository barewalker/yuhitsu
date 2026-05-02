#set text(lang: "en", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.")

#let business-report(
  title: "Business Report",
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
    [Author], [#author],
    [Department], [#affiliation],
    [Period], [#period],
    [Date], [#datetime.today().display()],
  )

  body
}

#show: business-report.with(
  title: "Business Report",
  author: "",
  affiliation: "",
  period: "",
)

= Activities
// List the activities carried out
-

= Progress / Outcomes

= Issues / Reflections

= Next Steps
