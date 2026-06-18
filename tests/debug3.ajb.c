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
block_0: // ic=4
    // #0 op=11 d=0 s1=0 s2=0 e=0
    t0 = p0;
    // #1 op=11 d=1 s1=1 s2=0 e=0
    t1 = p1;
    // #2 op=2 d=2 s1=0 s2=1 e=1
    t2 = t0 + t1;
    // #3 op=5 d=0 s1=2 s2=1 e=0
    return t2;

}

int main(void) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0: // ic=4
    // #0 op=2 d=1 s1=10 s2=0 e=1
    t1 = 10;
    // #1 op=2 d=2 s1=32 s2=0 e=1
    t2 = 32;
    // #2 op=4 d=0 s1=92 s2=3 e=3002
    t0 = add(t1, t2);
    // #3 op=5 d=0 s1=0 s2=1 e=0
    return t0;

}

