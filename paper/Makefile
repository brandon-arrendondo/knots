LATEX = pdflatex
FLAGS = -interaction=nonstopmode

PAPERS = survey aird

.PHONY: all clean $(PAPERS)

all: $(PAPERS)

$(PAPERS): %: %.pdf

%.pdf: %.tex
	$(LATEX) $(FLAGS) $<
	$(LATEX) $(FLAGS) $<

clean:
	rm -f *.aux *.log *.out *.fls *.fdb_latexmk *.pdf
