#set text(lang: "en", size: 11pt)
#set page(paper: "{{paper}}", margin: 2.5cm)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.")

#let meeting-minutes(
  title: "Meeting Minutes",
  location: "",
  attendees: "",
  recorder: "",
  body,
) = {
  align(center)[
    #text(size: 16pt, weight: "bold")[#title]
  ]

  v(1em)

  table(
    columns: (auto, 1fr),
    stroke: 0.5pt,
    inset: 6pt,
    [Date / Time], [#datetime.today().display() –],
    [Location], [#location],
    [Attendees], [#attendees],
    [Recorder], [#recorder],
  )

  body
}

#show: meeting-minutes.with(
  title: "Meeting Minutes",
  location: "",
  attendees: "",
  recorder: "",
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
