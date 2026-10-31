# Builds the product `zooagent` binary (framework + learning assistant plugin).
#
#   make                 -> debug -> bin/zooagent[.exe]
#   make build           -> same
#   make release         -> release
#   make run             -> run in web mode
#   make test            -> plugin workspace tests
#   make fmt             -> cargo fmt
#   make clean           -> remove bin/
#   make distclean       -> cargo clean + remove bin/
#
# The framework is the sibling checkout at `../agentzoo`. The product embeds
# that checkout's `zoo-server/web/dist`, so the frontend is built there first.

CARGO ?= cargo
FRAMEWORK := ../agentzoo
BIN_DIR := bin

ZVEC ?= 1
ZVEC_FEATURE := $(if $(filter 1,$(ZVEC)),--features zvec,)

ifeq ($(OS),Windows_NT)
  EXE := .exe
  FRONTEND_CMD := cmd /c "cd $(FRAMEWORK)/zoo-web && npm install && npm run build"
  ZVEC_DYLIB := zvec_c_api.dll
else
  EXE :=
  FRONTEND_CMD := cd $(FRAMEWORK)/zoo-web && npm install && npm run build
  ZVEC_OS := $(shell uname -s)
  ifeq ($(ZVEC_OS),Darwin)
    ZVEC_DYLIB := libzvec_c_api.dylib
  else
    ZVEC_DYLIB := libzvec_c_api.so
  endif
endif

.PHONY: all build release test run frontend fmt clean distclean stage-zvec

all: build

frontend:
	$(FRONTEND_CMD)

build: frontend
	$(CARGO) build --no-default-features $(ZVEC_FEATURE)
	mkdir -p $(BIN_DIR)
	cp target/debug/zooagent$(EXE) $(BIN_DIR)/zooagent$(EXE)
ifeq ($(ZVEC),1)
	$(MAKE) stage-zvec
endif

release: frontend
	$(CARGO) build --release --no-default-features $(ZVEC_FEATURE)
	mkdir -p $(BIN_DIR)
	cp target/release/zooagent$(EXE) $(BIN_DIR)/zooagent$(EXE)
ifeq ($(ZVEC),1)
	$(MAKE) stage-zvec
endif

# Reuse the framework's prebuilt zvec native library.
stage-zvec:
	$(MAKE) -C $(FRAMEWORK) fetch-zvec
	cp $(FRAMEWORK)/bin/$(ZVEC_DYLIB) $(BIN_DIR)/
	rm -rf $(BIN_DIR)/data && cp -r $(FRAMEWORK)/bin/data $(BIN_DIR)/data

test:
ifeq ($(ZVEC),1)
	$(MAKE) stage-zvec
	mkdir -p target/debug/deps
	cp $(BIN_DIR)/$(ZVEC_DYLIB) target/debug/deps/
	rm -rf target/debug/deps/data && cp -r $(BIN_DIR)/data target/debug/deps/data
endif
	$(CARGO) test --no-default-features $(ZVEC_FEATURE)

run:
	$(MAKE) build
	./$(BIN_DIR)/zooagent$(EXE) web

fmt:
	$(CARGO) fmt

clean:
	rm -rf $(BIN_DIR)

distclean: clean
	$(CARGO) clean
