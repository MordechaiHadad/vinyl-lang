local M = {}

local REPO_URL = "https://github.com/MordechaiHadad/vinyl-lang.git"
local RELEASE_URL = "https://github.com/MordechaiHadad/vinyl-lang/releases/latest/download/"
local LSP_BIN = "vinyl-lsp"

local function get_paths()
    local plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h:h")
    local is_windows = vim.uv.os_uname().sysname:match("Windows") ~= nil
    local bin_name = is_windows and (LSP_BIN .. ".exe") or LSP_BIN

    local plugin_bin_dir = plugin_dir .. "/bin"
    local plugin_local_bin = plugin_bin_dir .. "/" .. bin_name

    return {
        plugin_dir = plugin_dir,
        is_windows = is_windows,
        bin_name = bin_name,
        plugin_bin_dir = plugin_bin_dir,
        plugin_local_bin = plugin_local_bin,
    }
end

local function get_target_asset()
    local uname = vim.uv.os_uname()
    local sys = uname.sysname
    local arch = uname.machine

    if sys == "Linux" and (arch == "x86_64" or arch == "amd64") then
        return "vinyl-lsp-x86_64-unknown-linux-gnu.tar.gz"
    elseif sys == "Darwin" and (arch == "arm64" or arch == "aarch64") then
        return "vinyl-lsp-aarch64-apple-darwin.tar.gz"
    elseif sys:match("Windows") and (arch == "x86_64" or arch == "amd64") then
        return "vinyl-lsp-x86_64-pc-windows-msvc.zip"
    end

    return nil
end

local function download_binary(paths, install_dir, asset_name, callback)
    vim.fn.mkdir(install_dir, "p")
    local url = RELEASE_URL .. asset_name
    local target_bin = install_dir .. "/" .. paths.bin_name

    vim.notify("[vinyl.nvim] Downloading 'vinyl-lsp' prebuilt binary from GitHub Releases...", vim.log.levels.INFO)

    local cmd
    if paths.is_windows then
        local zip_file = install_dir .. "\\vinyl-lsp.zip"
        local ps_script = string.format(
            "Invoke-WebRequest -Uri '%s' -OutFile '%s'; Expand-Archive -Path '%s' -DestinationPath '%s' -Force; Remove-Item '%s'",
            url, zip_file, zip_file, install_dir, zip_file
        )
        cmd = { "powershell", "-NoProfile", "-Command", ps_script }
    else
        cmd = { "sh", "-c", string.format("curl -sL '%s' | tar -xz -C '%s'", url, install_dir) }
    end

    vim.system(cmd, { text = true }, function(obj)
        vim.schedule(function()
            if obj.code == 0 and vim.fn.executable(target_bin) == 1 then
                vim.notify("[vinyl.nvim] Successfully downloaded 'vinyl-lsp'!", vim.log.levels.INFO)
                callback(true, target_bin)
            else
                vim.notify("[vinyl.nvim] Binary download failed. Trying cargo fallback...", vim.log.levels.WARN)
                callback(false, nil)
            end
        end)
    end)
end

local function install_via_cargo(callback)
    if vim.fn.executable("cargo") == 0 then
        vim.notify(
            "[vinyl.nvim] Could not download prebuilt binary and 'cargo' is missing locally.",
            vim.log.levels.ERROR
        )
        callback(false, nil)
        return
    end

    vim.notify("[vinyl.nvim] Compiling 'vinyl-lsp' via Cargo...", vim.log.levels.INFO)
    local cmd = { "cargo", "install", "--git", REPO_URL, LSP_BIN }

    vim.system(cmd, { text = true }, function(obj)
        vim.schedule(function()
            if obj.code == 0 then
                vim.notify("[vinyl.nvim] Successfully installed 'vinyl-lsp' via Cargo!", vim.log.levels.INFO)
                callback(true, LSP_BIN)
            else
                vim.notify("[vinyl.nvim] Cargo installation failed:\n" .. (obj.stderr or ""), vim.log.levels.ERROR)
                callback(false, nil)
            end
        end)
    end)
end

local function ensure_lsp(callback)
    local paths = get_paths()

    -- 1. Check system PATH
    if vim.fn.executable(LSP_BIN) == 1 then
        callback(true, LSP_BIN)
        return
    end

    -- 2. Check local plugin bin directory
    if vim.fn.executable(paths.plugin_local_bin) == 1 then
        callback(true, paths.plugin_local_bin)
        return
    end

    -- 3. Prompt user for how to install
    local asset_name = get_target_asset()
    local choices = { "Cancel" }
    if asset_name then
        table.insert(choices, 1, "Download prebuilt release")
    end
    table.insert(choices, 2, "Compile via Cargo")

    vim.ui.select(choices, {
        prompt = "[vinyl.nvim] 'vinyl-lsp' not found. How do you want to install it?",
    }, function(choice)
        if not choice or choice == "Cancel" then
            vim.notify("[vinyl.nvim] LSP installation canceled.", vim.log.levels.WARN)
            callback(false, nil)
            return
        end

        if choice == "Download prebuilt release" then
            download_binary(paths, paths.plugin_bin_dir, asset_name, function(success, downloaded_bin)
                if success then
                    callback(true, downloaded_bin)
                else
                    install_via_cargo(callback)
                end
            end)
        else
            install_via_cargo(callback)
        end
    end)
end

function M.build_parser()
    local paths = get_paths()
    local parser_out = paths.plugin_dir .. "/parser"
    local src_dir = paths.plugin_dir .. "/.parser-src"

    vim.fn.mkdir(parser_out, "p")

    local function compile_parser()
        local grammar_dir = src_dir .. "/grammar"
        if vim.fn.isdirectory(grammar_dir) == 0 then
            vim.notify("[vinyl.nvim] Grammar directory not found at " .. grammar_dir, vim.log.levels.ERROR)
            return
        end

        if vim.fn.executable("tree-sitter") == 0 then
            vim.notify("[vinyl.nvim] 'tree-sitter' CLI is required to build the parser.", vim.log.levels.ERROR)
            return
        end

        local ext = paths.is_windows and ".dll" or ".so"
        local output_file = parser_out .. "/vinyl" .. ext
        local cmd = { "tree-sitter", "build", "--output", output_file, grammar_dir }

        vim.system(cmd, { text = true }, function(obj)
            vim.schedule(function()
                if obj.code == 0 then
                    vim.notify("[vinyl.nvim] Parser built successfully via tree-sitter CLI!", vim.log.levels.INFO)
                else
                    vim.notify(
                        "[vinyl.nvim] Parser build failed:\n" .. (obj.stderr or "Unknown error"),
                        vim.log.levels.ERROR
                    )
                end
            end)
        end)
    end

    if vim.fn.isdirectory(src_dir .. "/.git") == 1 then
        vim.system({ "git", "-C", src_dir, "pull" }, { text = true }, function()
            vim.schedule(compile_parser)
        end)
    else
        vim.system({ "git", "clone", "--depth", "1", REPO_URL, src_dir }, { text = true }, function()
            vim.schedule(compile_parser)
        end)
    end
end

function M.register_treesitter()
    vim.treesitter.language.register("vinyl", "vinyl")
end

function M.setup()
    M.register_treesitter()

    ensure_lsp(function(success, lsp_cmd)
        if not success or not lsp_cmd then
            return
        end

        local buf_path = vim.api.nvim_buf_get_name(0)
        local buf_dir = buf_path ~= "" and vim.fs.dirname(buf_path) or vim.fn.getcwd()
        local root_dir = vim.fs.root(0, { ".git" }) or buf_dir

        vim.lsp.start({
            name = "vinyl-lsp",
            cmd = { lsp_cmd },
            root_dir = root_dir,
        })
    end)
end

return M
