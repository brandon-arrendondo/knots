# Sphinx configuration for Knots User Guide

project = 'Knots'
author = 'Brandon Arrendondo'
copyright = '2026, Brandon Arrendondo'

extensions = []

templates_path = []
exclude_patterns = []

# -- HTML output (sphinx-rtd-theme) --
html_theme = 'sphinx_rtd_theme'
html_static_path = []

# -- LaTeX / PDF output --
latex_elements = {
    'papersize': 'letterpaper',
    'pointsize': '10pt',
    'preamble': r'''
\usepackage{enumitem}
\setlistdepth{9}
''',
}

latex_documents = [
    ('index', 'knots-user-guide.tex', 'Knots User Guide',
     'Brandon Arrendondo', 'manual'),
]
