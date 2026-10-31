# Builds the product `zooagent` binary (framework libs + learning assistant app).
#
#   make                 -> debug -> bin/zooagent[.exe]
#   make build           -> same
#   make release         -> release
#   make run             -> run in web mode
#   make test            -> app workspace tests
#   make fmt             -> cargo fmt
#   make clean           -> remove bin/
#   make distclean       -> cargo clean + remove bin/
#
# Framework crates are consumed from the private git repository (see
# Cargo.toml), not from a sibling checkout.

CARGO ?= cargo
BIN_DIR := bin

ZVEC ?= 0
ZVEC_FEATURE := $(if $(filter 1,$(ZVEC)),--features zvec,)

ifeq ($(OS),Windows_NT)
  EXE := .exe
  PRODUCT_WEB_CMD := cmd /c "cd host-web && npm install && npm run build"
  PLUGIN_FRONTEND_CMD := cmd /c "cd assistant-web && npm install && npm run build"
else
  EXE :=
  PRODUCT_WEB_CMD := cd host-web && npm install && npm run build
  PLUGIN_FRONTEND_CMD := cd assistant-web && npm install && npm run build
endif

.PHONY: all build release test run frontend plugin-frontend product-web fmt clean distclean

all: build

frontend: plugin-frontend product-web

plugin-frontend:
	$(PLUGIN_FRONTEND_CMD)

product-web:
	$(PRODUCT_WEB_CMD)

build: frontend
	$(CARGO) build --no-default-features $(ZVEC_FEATURE)
	mkdir -p $(BIN_DIR)
	cp target/debug/zooagent$(EXE) $(BIN_DIR)/zooagent$(EXE)
	rm -rf $(BIN_DIR)/host-web && cp -r host-web/dist $(BIN_DIR)/host-web

release: frontend
	$(CARGO) build --release --no-default-features $(ZVEC_FEATURE)
	mkdir -p $(BIN_DIR)
	cp target/release/zooagent$(EXE) $(BIN_DIR)/zooagent$(EXE)
	rm -rf $(BIN_DIR)/host-web && cp -r host-web/dist $(BIN_DIR)/host-web

test:
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
