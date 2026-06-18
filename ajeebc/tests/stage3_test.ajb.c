#include <stdint.h>
#include <string.h>
#include <stdio.h>

intptr_t getStateBuf(void);
intptr_t getOutbuf(void);
intptr_t getInt(intptr_t, intptr_t);
void setInt(intptr_t, intptr_t, intptr_t);
intptr_t len(intptr_t);
void strSet(intptr_t, intptr_t, intptr_t);
intptr_t charCode(intptr_t, intptr_t);
intptr_t str_concat(intptr_t, intptr_t);
intptr_t substring(intptr_t, intptr_t, intptr_t);
intptr_t indexOf(intptr_t, intptr_t);
intptr_t contains(intptr_t, intptr_t);
intptr_t itoa(intptr_t);
intptr_t println(intptr_t);
intptr_t readArg(intptr_t);
intptr_t readFile(intptr_t);
void writeFile(intptr_t, intptr_t);
void writeAppend(intptr_t, intptr_t);
void writeByte(intptr_t, intptr_t);
intptr_t array_to_string(intptr_t, intptr_t);

char __ajeeb_buf[16384];
char __ajeeb_outbuf[65536];

intptr_t add(intptr_t, intptr_t);

intptr_t add(intptr_t p0, intptr_t p1) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0:
    t0 = p0;
    t1 = p1;
    t2 = t0 + t1;
    return t2;

}

int main(void) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0:
    t1 = 10;
    t2 = 32;
    t0 = add(t1, t2);
    return t0;

}

