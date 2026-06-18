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


int main(void) {
    intptr_t t0, t1, t2, t3, t4, t5, t6, t7, t8, t9;

    goto block_0;
block_0: // ic=1
    // #0 op=2 d=0 s1=0 s2=0 e=1
    t0 = 0;

block_1: // ic=3
    // #0 op=2 d=1 s1=10 s2=0 e=1
    t1 = 10;
    // #1 op=2 d=2 s1=0 s2=1 e=7
    t2 = t0 < t1;
    // #2 op=7 d=2 s1=2 s2=5 e=0
    if (t2) { goto block_2; } else { goto block_5; }

block_2: // ic=3
    // #0 op=2 d=3 s1=6 s2=0 e=1
    t3 = 6;
    // #1 op=2 d=4 s1=0 s2=3 e=5
    t4 = t0 == t3;
    // #2 op=7 d=4 s1=3 s2=4 e=0
    if (t4) { goto block_3; } else { goto block_4; }

block_3: // ic=1
    // #0 op=6 d=0 s1=4 s2=0 e=0
    goto block_4;

block_4: // ic=6
    // #0 op=4 d=6 s1=150 s2=4 e=1
    t6 = itoa(t0);
    // #1 op=4 d=5 s1=142 s2=7 e=7
    t5 = println(t6);
    // #2 op=2 d=7 s1=1 s2=0 e=1
    t7 = 1;
    // #3 op=2 d=8 s1=0 s2=7 e=1
    t8 = t0 + t7;
    // #4 op=2 d=0 s1=8 s2=0 e=101
    t0 = t8;
    // #5 op=6 d=0 s1=1 s2=0 e=0
    goto block_1;

block_5: // ic=2
    // #0 op=2 d=9 s1=0 s2=0 e=1
    t9 = 0;
    // #1 op=5 d=0 s1=9 s2=1 e=0
    return t9;

}

