#set text(lang: "en", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.")

#align(center)[
  #text(size: 16pt, weight: "bold")[Business Report]
]

#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 6pt,
  [Author], [],
  [Department], [],
  [Period], [#datetime.today().display("[year]/[month]/[day]") –],
  [Date], [#datetime.today().display()],
)

= Activities
// List the activities carried out
-

= Progress / Outcomes

= Issues / Reflections

= Next Steps
