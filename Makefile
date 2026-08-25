# Amberlink Build System
# Usage:
#   make init          - Build the toolchain (compiler + VM)
#   make build file=X  - Compile source.amb to bytecode
#   make run file=X    - Compile and run (or just run .amc)
#   make clean         - Remove build artifacts

# OS detection
ifeq ($(OS),Windows_NT)
    EXE := .exe
    CP  := copy
else
    EXE :=
    CP  := cp
endif

# Paths
CORE_DIR  := amber-core
VM_DIR    := amber-vm
BUILD_DIR := $(VM_DIR)/build
BIN_DIR   := bin
COMPILER  := $(BIN_DIR)/ambc$(EXE)
VM        := $(BIN_DIR)/avm$(EXE)

# Rust binary name differs by platform
RUST_BIN  := $(CORE_DIR)/target/release/amber-core$(EXE)

.PHONY: init build run clean

init: $(COMPILER) $(VM)

$(COMPILER):
	@echo "Compiling Rust Core..."
	cd $(CORE_DIR) && cargo build --release
	@mkdir -p $(BIN_DIR)
	$(CP) $(RUST_BIN) $(COMPILER)

$(VM):
	@echo "Compiling C++ VM..."
	@mkdir -p $(BUILD_DIR)
	cd $(BUILD_DIR) && cmake .. && cmake --build . --config Release
	@mkdir -p $(BIN_DIR)
	$(CP) $(BUILD_DIR)/avm$(EXE) $(VM)

build: $(COMPILER)
	$(COMPILER) $(file)

run: $(COMPILER) $(VM)
	@if echo "$(file)" | grep -q '\.amb$$'; then \
		$(COMPILER) $(file) || exit 1; \
		$(VM) $$(echo $(file) | sed 's/\.amb$$/.amc/'); \
	else \
		$(VM) $(file); \
	fi

clean:
	cd $(CORE_DIR) && cargo clean
	rm -rf $(BUILD_DIR)
	rm -rf $(BIN_DIR)
