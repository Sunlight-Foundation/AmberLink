# Amberlink Build System
# Usage:
#   make init          - Build the toolchain (compiler + VM)
#   make build file=X  - Compile source.amb to bytecode
#   make run file=X    - Compile and run (.amb), or run directly (.amc/.ama)
#   make new name=X    - Scaffold a new project in directory X
#   make test          - Compile + run every example (regression suite)
#   make bench         - Run benchmarks (bench/*.amb, self-timed via clock())
#   make watch file=X  - Rebuild and rerun on every *.amb change
#   make clean         - Remove build artifacts

# OS & Shell detection
ifeq ($(OS),Windows_NT)
    EXE := .exe
    # Check if Make is running inside CMD/PowerShell vs a Unix shell (Bash/Git Bash/MSYS)
    ifeq ($(findstring sh,$(SHELL)),)
        MKDIR = @if not exist "$(1)" mkdir "$(1)"
        COPY  = @copy "$(1)" "$(2)" >nul
        COPYDIR = @xcopy /E /I /Q "$(1)" "$(2)" >nul
        RMDIR = @if exist "$(1)" rmdir /s /q "$(1)"
        IS_CMD = 1
    else
        MKDIR = @mkdir -p "$(1)"
        COPY  = @cp "$(1)" "$(2)"
        COPYDIR = @cp -r "$(1)/." "$(2)/"
        RMDIR = @rm -rf "$(1)"
    endif
else
    EXE :=
    MKDIR = @mkdir -p "$(1)"
    COPY  = @cp "$(1)" "$(2)"
    COPYDIR = @cp -r "$(1)/." "$(2)/"
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

.PHONY: init build run clean new test bench watch

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
ifndef file
	$(error Usage: make build file=main.amb)
endif
	$(COMPILER) $(file)

# .amb files: compile then run; .amc/.ama files: run directly
AMB_SRC := $(filter %.amb,$(file))
run: $(COMPILER) $(VM)
ifndef file
	$(error Usage: make run file=main.amb)
endif
ifdef AMB_SRC
	$(COMPILER) $(file)
	$(VM) $(subst .amb,.amc,$(file))
else
	$(VM) $(file)
endif

# Scaffold a new project from tools/template
new:
ifndef name
	$(error Usage: make new name=myproject)
endif
	$(call MKDIR,$(name))
	$(call COPYDIR,tools/template,$(name))
	@echo "Created project in $(name)/"

# Full regression suite (compiles + runs every example)
test: $(COMPILER) $(VM)
ifdef IS_CMD
	@powershell -NoProfile -ExecutionPolicy Bypass -File tools/test.ps1
else
	@bash tools/test.sh
endif

# Run benchmarks (each bench prints its result + elapsed seconds)
bench: $(COMPILER) $(VM)
ifdef IS_CMD
	@powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench.ps1
else
	@bash tools/bench.sh
endif

# Rebuild and rerun whenever any *.amb changes
watch: $(COMPILER) $(VM)
ifndef file
	$(error Usage: make watch file=main.amb)
endif
ifdef IS_CMD
	@powershell -NoProfile -ExecutionPolicy Bypass -File tools/watch.ps1 -file "$(file)"
else
	@bash tools/watch.sh "$(file)"
endif

clean:
	cd $(CORE_DIR) && cargo clean
	$(call RMDIR,$(BUILD_DIR))
	$(call RMDIR,$(BIN_DIR))