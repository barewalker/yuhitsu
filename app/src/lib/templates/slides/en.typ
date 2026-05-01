// Paper size setting ({{paper}}) is unused in this template; slides are fixed 16:9.
// Will be redesigned when integrating touying / polylux in the future.
#set text(lang: "en", size: 22pt)
#set page(paper: "presentation-16-9", margin: 2cm)
#set par(leading: 0.7em)

// Title slide
#align(center + horizon)[
  #text(size: 40pt, weight: "bold")[Presentation Title]
  #v(1em)
  #text(size: 24pt)[Subtitle]
  #v(2em)
  #text(size: 18pt)[Presenter Name]
  #v(0.5em)
  #text(size: 16pt)[#datetime.today().display()]
]

#pagebreak()

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
