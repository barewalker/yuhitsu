// Paper size setting ({{paper}}) is unused in this template; slides are fixed 16:9.
// Will be redesigned when integrating touying / polylux in the future.
#set text(lang: "en", size: 22pt)
#set page(paper: "presentation-16-9", margin: 2cm)
#set par(leading: 0.7em)

#let slides(
  title: "Presentation Title",
  subtitle: "",
  presenter: "",
  body,
) = {
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
  title: "Presentation Title",
  subtitle: "Subtitle",
  presenter: "Presenter Name",
)

// Table of contents
= Outline
+ Introduction
+ Main Discussion
+ Summary

#pagebreak()

= Introduction
- Background
- Objective
- Problem

#pagebreak()

= Main Discussion
- Point 1
- Point 2
- Point 3

#pagebreak()

= Summary
- Conclusion
- Future Work

#pagebreak()

#align(center + horizon)[
  #text(size: 48pt, weight: "bold")[Thank You]
]
