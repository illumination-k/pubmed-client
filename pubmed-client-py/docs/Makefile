# Minimal Sphinx makefile

SPHINXBUILD   ?= uv run --group docs sphinx-build
SOURCEDIR     = source
BUILDDIR      = build

.PHONY: help html clean

help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

html:
	$(SPHINXBUILD) -b html "$(SOURCEDIR)" "$(BUILDDIR)/html" $(SPHINXOPTS) $(O)
	@echo "Build finished. Open build/html/index.html to view."

clean:
	rm -rf "$(BUILDDIR)"
