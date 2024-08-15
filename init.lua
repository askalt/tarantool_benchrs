local ffi = require("ffi")
local os = require("os")

box.cfg {
    work_dir = "./data",
    memtx_memory = 512 * 1024 * 1024,
}

box.schema.create_space("bindata", {
    engine = "memtx",
    field_count = 2,
    format = {{"id", "unsigned"}, {"data", "varbinary"}},
    if_not_exists = true
})
box.space.bindata:create_index("pk", {
    if_not_exists = true
})

-- Debug spaces.
if os.getenv("CONSOLE") then
    require("console").start()
    return
end

local lib_path = os.getenv("LIBPATH")
local lib = ffi.load(lib_path)

ffi.cdef [[
    typedef struct {
        uint64_t retries;
        bool verbose;
        uint64_t method;
        uint64_t block_size;
        uint64_t block_num;
        float update_percentage;
        bool transaction_per_block;
    } Config;

    void run(Config);
]]

-- 0 - copy
-- 1 - splices
local method = 1
local verbose = false
local block_size = 10000
local block_num = 100
local transaction_per_block = false

local update_entries = 1
while update_entries < block_size do
    box.space.bindata:truncate()
    local config = {
        retries = 15,
        verbose = verbose,
        method = method,
        block_size = block_size,
        block_num = block_num,
        update_percentage = update_entries / block_size,
        transaction_per_block = transaction_per_block
    }
    lib.run(config)

    -- For small count we want to see more.
    if update_entries < 100 then
        update_entries = update_entries + 5
    else
        update_entries = update_entries + 50
    end
end

-- local config = {
--     retries = 15,
--     method = 1,
--     block_size = 10000,
--     block_num = 100,
--     update_percentage = 0.01,
--     transaction_per_column = false,
-- }

-- lib.run(config)

os.exit(0)
