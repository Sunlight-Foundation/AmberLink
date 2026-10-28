// fficmp.c - FFI array test fixture for examples/ffi_buf_test.amb.
// Build (done automatically by tools/test when a C toolchain exists):
//   Windows: gcc -shared -o bin/fficmp.dll examples/resources/fficmp.c
//   POSIX:   cc -shared -fPIC -o bin/libfficmp.so examples/resources/fficmp.c
int asum(int* data, int len) {
    int s = 0;
    for (int i = 0; i < len; i++) s += data[i];
    return s;
}
int ascale(int* data, int len) {
    for (int i = 0; i < len; i++) data[i] *= 10;
    return len;
}
