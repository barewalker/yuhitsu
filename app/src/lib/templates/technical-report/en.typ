#set text(lang: "en", size: 10.5pt)
#set page(paper: "{{paper}}", margin: 2.5cm, numbering: "1")
#set par(justify: true, leading: 0.65em, first-line-indent: 1em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(size: 14pt)
#show heading.where(level: 2): set text(size: 12pt)

#let technical-report(
  title: "Technical Report Title",
  author: "",
  affiliation: "",
  abstract: "",
  body,
) = {
  set document(title: title, author: author)
  align(center)[
    #text(size: 18pt, weight: "bold")[#title]
    #v(0.5em)
    #if affiliation != "" [#author / #affiliation] else [#author]
    #v(0.3em)
    #datetime.today().display()
  ]

  v(1em)

  if abstract != "" {
    align(center)[
      #box(width: 90%)[
        #set par(first-line-indent: 0pt)
        *Abstract* — #abstract
      ]
    ]
    v(1em)
  }

  body
}

#show: technical-report.with(
  title: "Technical Report Title",
  author: "",
  affiliation: "",
  abstract: "Write a concise summary (around 200 words) covering background, objectives, methods, and key results.",
)

= Introduction
Background, prior work, and objectives.

= Methods
Materials, conditions, and procedures in enough detail for reproducibility.

= Results
Measurements, figures, and tables.

// Figure example
// #figure(
//   image("figure1.png", width: 80%),
//   caption: [Figure 1: Experimental setup],
// )

= Discussion
Interpretation, comparison with prior work, and limitations.

= Conclusion
Findings and future directions.

// Bibliography supports Hayagriva YAML (.yml) and BibTeX (.bib).
// Hayagriva is Typst-native with an intuitive structure (recommended).
// BibTeX is the academic standard and reuses existing assets.
// Uncomment after creating the file (the "Insert bibliography" toolbar command
// also picks a file via the dialog and inserts the line automatically).
// #bibliography("references.yml")
// #bibliography("references.bib")
