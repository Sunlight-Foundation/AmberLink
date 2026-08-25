# Amberlink.py
# A simple script tool to easily build and run Amber projects, U can also use this to initialize the build process

import sys
import compile
import os
import subprocess

def get_bin_paths():
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    bin_dir = os.path.join(root_dir, "bin")
    compiler_name = "ambc.exe" if os.name == "nt" else "ambc"
    avm_name = "avm.exe" if os.name == "nt" else "avm"
    return bin_dir, os.path.join(bin_dir, compiler_name), os.path.join(bin_dir, avm_name)

def main():
    if len(sys.argv) < 2:
        print("Usage: python Amberlink.py [init|build <file>|run <file>|install]")
        return

    command = sys.argv[1].lower()
    
    if command == "init":
        compile.build()

    elif command == "build":
        if len(sys.argv) < 3:
            print("Usage: python Amberlink.py build <file.amb>")
            return

        filename = sys.argv[2]
        bin_dir, compiler_path, _ = get_bin_paths()

        if not os.path.exists(compiler_path):
            print(f"Error: Compiler not found at {compiler_path}")
            print("Run 'python Amberlink.py init' first.")
            return

        subprocess.run([compiler_path, filename])

    elif command == "run":
        if len(sys.argv) < 3:
            print("Usage: python Amberlink.py run <file.amb|file.amc>")
            return

        filename = sys.argv[2]
        bin_dir, compiler_path, avm_path = get_bin_paths()

        if not os.path.exists(avm_path):
            print(f"Error: AVM not found at {avm_path}")
            print("Run 'python Amberlink.py init' first.")
            return

        # If .amb, compile first then run the resulting .amc
        if filename.endswith(".amb"):
            if not os.path.exists(compiler_path):
                print(f"Error: Compiler not found at {compiler_path}")
                print("Run 'python Amberlink.py init' first.")
                return
            result = subprocess.run([compiler_path, filename])
            if result.returncode != 0:
                print("Compilation failed.")
                return
            filename = filename.replace(".amb", ".amc")

        if not filename.endswith(".amc"):
            print("Error: Expected a .amb or .amc file.")
            return

        if not os.path.exists(filename):
            print(f"Error: File not found: {filename}")
            return

        subprocess.run([avm_path, filename])

    elif command == "install":
        print("This option isn't done yet")

    else:
        print(f"Unknown command: {command}")

if __name__ == "__main__":
    main()