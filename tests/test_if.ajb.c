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
    intptr_t t0, t1, t2, t3, t4, t5, t6, t7;

    goto block_0;
block_0: // ic=4
    // #0 op=2 d=0 s1=10 s2=0 e=1
    t0 = 10;
    // #1 op=2 d=1 s1=5 s2=0 e=1
    t1 = 5;
    // #2 op=2 d=2 s1=0 s2=1 e=8
    t2 = t0 > t1;
    // #3 op=7 d=2 s1=1 s2=2 e=0
    if (t2) { goto block_1; } else { goto block_2; }

block_1: // ic=3
    // #0 op=2 d=4 s1=70 s2=8 e=2
    t4 = (intptr_t)"bada hai";
    // #1 op=4 d=3 s1=61 s2=7 e=5
    t3 = println(t4);
    // #2 op=6 d=0 s1=3 s2=0 e=0
    goto block_3;

block_2: // ic=3
    // #0 op=2 d=6 s1=104 s2=10 e=2
    t6 = (intptr_t)"chhota hai";
    // #1 op=4 d=5 s1=95 s2=7 e=7
    t5 = println(t6);
    // #2 op=6 d=0 s1=3 s2=0 e=0
    goto block_3;

block_3: // ic=2
    // #0 op=2 d=7 s1=0 s2=0 e=1
    t7 = 0;
    // #1 op=5 d=0 s1=7 s2=1 e=0
    return t7;

}

