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
intptr_t strcmp_ajeeb(intptr_t, intptr_t);
intptr_t trim(intptr_t);
intptr_t toUpperCase(intptr_t);
intptr_t toLowerCase(intptr_t);
intptr_t startsWith(intptr_t, intptr_t);
intptr_t endsWith(intptr_t, intptr_t);
intptr_t replace(intptr_t, intptr_t, intptr_t);
intptr_t array_to_string(intptr_t, intptr_t);

char __ajeeb_buf[16384];
char __ajeeb_outbuf[65536];

intptr_t greet(intptr_t);

intptr_t greet(intptr_t p0) {
    intptr_t t0, t1, t2, t3;

    goto block_0;
block_0: // ic=4
    // #0 op=11 d=0 s1=0 s2=0 e=0
    t0 = p0;
    // #1 op=2 d=2 s1=64 s2=6 e=2
    t2 = (intptr_t)"Hello ";
    // #2 op=12 d=3 s1=2 s2=0 e=0
    t3 = str_concat(t2, t0);
    // #3 op=4 d=1 s1=55 s2=7 e=4
    t1 = println(t3);

}

int main(void) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0: // ic=4
    // #0 op=2 d=1 s1=117 s2=5 e=2
    t1 = (intptr_t)"World";
    // #1 op=4 d=0 s1=110 s2=5 e=2
    t0 = greet(t1);
    // #2 op=2 d=2 s1=0 s2=0 e=1
    t2 = 0;
    // #3 op=5 d=0 s1=2 s2=1 e=0
    return t2;

}

