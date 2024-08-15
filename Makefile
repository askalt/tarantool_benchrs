all: run

LIBPATH = ./target/debug/libupdate_benchmark.so
TARANTOOL = tarantool

.PHONY: data_dir
data_dir:
	mkdir -p data

.PHONY: build
build:
	cargo build

.PHONY: run
run: build data_dir
	LIBPATH=$(realpath $(LIBPATH)) $(TARANTOOL) init.lua

.PHONY: build data_dir
run_massif:
	LIBPATH=$(realpath $(LIBPATH)) valgrind --tool=massif $(TARANTOOL) init.lua

.PHONY: clean
clean:
	rm -rf data

.PHONY: console
console:
	CONSOLE=1 $(TARANTOOL) init.lua
