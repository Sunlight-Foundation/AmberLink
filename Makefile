# Amberlink Build System
# Usage:
#   make init          - Build the toolchain (compiler + VM)
#   make build file=X  - Compile source.amb to bytecode
#   make run file=X    - Compile and run (or just run .amc)
#   make clean         - Remove build artifacts

# OS & Shell detection
ifeq ($(OS),Windows_NT)
    EXE := .exe
    # Check if Make is running inside CMD/PowerShell vs a Unix shell (Bash/Git Bash/MSYS)
    ifeq ($(findstring sh,$(SHELL)),)
        MKDIR = @if not exist "$(1)" mkdir "$(1)"
        COPY  = @copy "$(1)" "$(2)" >nul
        RMDIR = @if exist "$(1)" rmdir /s /q "$(1)"
    else
        MKDIR = @mkdir -p "$(1)"
        COPY  = @cp "$(1)" "$(2)"
        RMDIR = @rm -rf "$(1)"
    endif
else
    EXE :=
    MKDIR = @mkdir -p "$(1)"
    COPY  = @cp "$(1)" "$(2)"
    RMDIR = @rm -rf "$(1)"
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
	$(call MKDIR,$(BIN_DIR))
	$(call COPY,$(RUST_BIN),$(COMPILER))

$(VM):
	@echo "Compiling C++ VM..."
	$(call MKDIR,$(BUILD_DIR))
	cd $(BUILD_DIR) && cmake .. && cmake --build . --config Release
	$(call MKDIR,$(BIN_DIR))
	$(call COPY,$(BUILD_DIR)/avm$(EXE),$(VM))

build: $(COMPILER)
	$(COMPILER) $(file)

# .amb files: compile then run; .amc files: run directly
AMB_SRC := $(filter %.amb,$(file))
run: $(COMPILER) $(VM)
ifdef AMB_SRC
	$(COMPILER) $(file)
	$(VM) $(subst .amb,.amc,$(file))
else
	$(VM) $(file)
endif

clean:
	cd $(CORE_DIR) && cargo clean
	$(call RMDIR,$(BUILD_DIR))
	$(call RMDIR,$(BIN_DIR))