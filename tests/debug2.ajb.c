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


int main(void) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0: // ic=4
    // #0 op=2 d=0 s1=10 s2=0 e=1
    t0 = 10;
    // #1 op=2 d=1 s1=20 s2=0 e=1
    t1 = 20;
    // #2 op=2 d=2 s1=0 s2=1 e=1
    t2 = t0 + t1;
    // #3 op=5 d=0 s1=2 s2=1 e=0
    return t2;

}

