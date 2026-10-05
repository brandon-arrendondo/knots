# probe: `#` inside a string is not a comment
# source: Park 1992 (physical lines holding code); the Python Language Reference, "Comments": a comment starts with
#         a hash character that is not part of a string literal. Every line of f holds code: 5.
# expect f sloc=5
def f():
    s = """
# not a comment
"""
    return s
