-- Buffer-local options for Vinyl files
vim.bo.commentstring = "# %s"
vim.bo.shiftwidth = 4
vim.bo.tabstop = 4
vim.bo.expandtab = true

-- Start vinyl-lsp automatically
vim.lsp.start({
  name = "vinyl-lsp",
  cmd = { "vinyl-lsp" },
  root_dir = vim.fs.root(0, { ".git" }) or vim.fs.dirname(vim.api.nvim_buf_get_name(0)),
})
