-- Buffer-local options for Vinyl files
vim.bo.commentstring = "# %s"
vim.bo.shiftwidth = 4
vim.bo.tabstop = 4
vim.bo.expandtab = true
vim.bo.indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
vim.bo.indentkeys = "0{,0},0),0],0#,!^F,o,O,e"

-- Safely activate Treesitter highlighting
pcall(vim.treesitter.start)

-- Call lua/vinyl/init.lua -> setup()
require("vinyl").setup()
