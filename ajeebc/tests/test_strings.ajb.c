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
    intptr_t t0, t1, t2, t3, t4, t5, t6, t7, t8, t9, t10, t11, t12, t13, t14, t15, t16, t17, t18, t19, t20, t21, t22, t23, t24;

    goto block_0;
block_0: // ic=26
    // #0 op=2 d=0 s1=44 s2=15 e=2
    t0 = (intptr_t)"  Hello World  ";
    // #1 op=4 d=2 s1=74 s2=4 e=1
    t2 = trim(t0);
    // #2 op=4 d=1 s1=66 s2=7 e=3
    t1 = println(t2);
    // #3 op=2 d=5 s1=109 s2=5 e=2
    t5 = (intptr_t)"hello";
    // #4 op=4 d=4 s1=96 s2=11 e=6
    t4 = toUpperCase(t5);
    // #5 op=4 d=3 s1=88 s2=7 e=5
    t3 = println(t4);
    // #6 op=2 d=8 s1=144 s2=5 e=2
    t8 = (intptr_t)"AJEEB";
    // #7 op=4 d=7 s1=131 s2=11 e=9
    t7 = toLowerCase(t8);
    // #8 op=4 d=6 s1=123 s2=7 e=8
    t6 = println(t7);
    // #9 op=2 d=12 s1=180 s2=5 e=2
    t12 = (intptr_t)"Hello";
    // #10 op=2 d=13 s1=189 s2=3 e=2
    t13 = (intptr_t)"ell";
    // #11 op=4 d=11 s1=171 s2=7 e=14013
    t11 = indexOf(t12, t13);
    // #12 op=4 d=10 s1=166 s2=4 e=12
    t10 = itoa(t11);
    // #13 op=4 d=9 s1=158 s2=7 e=11
    t9 = println(t10);
    // #14 op=2 d=17 s1=225 s2=5 e=2
    t17 = (intptr_t)"Hello";
    // #15 op=2 d=18 s1=234 s2=3 e=2
    t18 = (intptr_t)"ell";
    // #16 op=4 d=16 s1=215 s2=8 e=19018
    t16 = contains(t17, t18);
    // #17 op=4 d=15 s1=210 s2=4 e=17
    t15 = itoa(t16);
    // #18 op=4 d=14 s1=202 s2=7 e=16
    t14 = println(t15);
    // #19 op=2 d=21 s1=266 s2=11 e=2
    t21 = (intptr_t)"Hello World";
    // #20 op=2 d=22 s1=0 s2=0 e=1
    t22 = 0;
    // #21 op=2 d=23 s1=5 s2=0 e=1
    t23 = 5;
    // #22 op=4 d=20 s1=255 s2=9 e=24023022
    t20 = substring(t21, t22, t23);
    // #23 op=4 d=19 s1=247 s2=7 e=21
    t19 = println(t20);
    // #24 op=2 d=24 s1=0 s2=0 e=1
    t24 = 0;
    // #25 op=5 d=0 s1=24 s2=1 e=0
    return t24;

}

