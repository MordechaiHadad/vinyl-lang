TESTSUBJECT=$1
tree-sitter generate
tree-sitter parse $TESTSUBJECT
