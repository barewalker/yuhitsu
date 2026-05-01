#set text(lang: "en", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.")

#align(center)[
  #text(size: 16pt, weight: "bold")[Meeting Minutes]
]

#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt,
  inset: 6pt,
  [Date / Time], [#datetime.today().display() –],
  [Location], [],
  [Attendees], [],
  [Recorder], [],
)

= Agenda
+

= Discussion

= Decisions
-

= Action Items
#table(
  columns: (1fr, auto, auto),
  stroke: 0.5pt,
  inset: 6pt,
  [Action], [Owner], [Due],
  [], [], [],
)
