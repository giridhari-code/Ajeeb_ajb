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
intptr_t factorial(intptr_t);

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

intptr_t factorial(intptr_t p0) {
    intptr_t t0, t1, t2, t3, t4, t5, t6, t7;

    goto block_0;
block_0: // ic=4
    // #0 op=11 d=0 s1=0 s2=0 e=0
    t0 = p0;
    // #1 op=2 d=1 s1=1 s2=0 e=1
    t1 = 1;
    // #2 op=2 d=2 s1=0 s2=1 e=9
    t2 = t0 <= t1;
    // #3 op=7 d=2 s1=1 s2=2 e=0
    if (t2) { goto block_1; } else { goto block_2; }

block_1: // ic=2
    // #0 op=2 d=3 s1=1 s2=0 e=1
    t3 = 1;
    // #1 op=5 d=0 s1=3 s2=1 e=0
    return t3;

block_2: // ic=3
    // #0 op=5 d=14 s1=314 s2=3 e=11
    return t314;
    // #1 op=4 d=1 s1=690 s2=0 e=11
    t1 = (t10);
    // #2 op=7 d=2 s1=690 s2=693 e=0
    if (t2) { goto block_690; } else { goto block_693; }

}

int main(void) {
    intptr_t t0, t1, t2, t3, t4, t5, t6, t7, t8, t9, t10, t11, t12, t13, t14, t15, t16, t17, t18, t19, t20, t21, t22, t23, t24, t25, t26, t27;

    goto block_0;
block_0: // ic=18
    // #0 op=2 d=0 s1=10 s2=0 e=1
    t0 = 10;
    // #1 op=2 d=1 s1=20 s2=0 e=1
    t1 = 20;
    // #2 op=4 d=2 s1=277 s2=3 e=2001
    t2 = add(t0, t1);
    // #3 op=4 d=4 s1=309 s2=4 e=3
    t4 = itoa(t2);
    // #4 op=4 d=3 s1=292 s2=7 e=5003
    t3 = println(t2, t4);
    // #5 op=2 d=6 s1=5 s2=0 e=1
    t6 = 5;
    // #6 op=4 d=5 s1=334 s2=9 e=7
    t5 = factorial(t6);
    // #7 op=4 d=8 s1=378 s2=4 e=6
    t8 = itoa(t5);
    // #8 op=4 d=7 s1=352 s2=7 e=9006
    t7 = println(t5, t8);
    // #9 op=2 d=9 s1=403 s2=5 e=2
    t9 = (intptr_t)"Hello";
    // #10 op=2 d=10 s1=425 s2=5 e=2
    t10 = (intptr_t)"World";
    // #11 op=2 d=11 s1=452 s2=1 e=2
    t11 = (intptr_t)" ";
    // #12 op=12 d=12 s1=9 s2=11 e=0
    t12 = str_concat(t9, t11);
    // #13 op=2 d=13 s1=12 s2=10 e=1
    t13 = t12 + t10;
    // #14 op=4 d=14 s1=465 s2=7 e=14
    t14 = println(t13);
    // #15 op=2 d=15 s1=0 s2=0 e=1
    t15 = 0;
    // #16 op=2 d=16 s1=0 s2=0 e=1
    t16 = 0;
    // #17 op=2 d=17 s1=0 s2=0 e=1
    t17 = 0;

block_1: // ic=3
    // #0 op=2 d=18 s1=5 s2=0 e=1
    t18 = 5;
    // #1 op=2 d=19 s1=17 s2=18 e=7
    t19 = t17 < t18;
    // #2 op=7 d=19 s1=2 s2=3 e=0
    if (t19) { goto block_2; } else { goto block_3; }

block_2: // ic=6
    // #0 op=2 d=20 s1=16 s2=17 e=1
    t20 = t16 + t17;
    // #1 op=2 d=16 s1=20 s2=0 e=101
    t16 = t20;
    // #2 op=2 d=21 s1=1 s2=0 e=1
    t21 = 1;
    // #3 op=2 d=22 s1=17 s2=21 e=1
    t22 = t17 + t21;
    // #4 op=2 d=17 s1=22 s2=0 e=101
    t17 = t22;
    // #5 op=6 d=1 s1=0 s2=0 e=0
    goto block_0;

block_3: // ic=6
    // #0 op=4 d=24 s1=606 s2=4 e=17
    t24 = itoa(t16);
    // #1 op=4 d=23 s1=584 s2=7 e=25017
    t23 = println(t16, t24);
    // #2 op=2 d=26 s1=632 s2=4 e=2
    t26 = (intptr_t)"DONE";
    // #3 op=4 d=25 s1=623 s2=7 e=27
    t25 = println(t26);
    // #4 op=2 d=27 s1=0 s2=0 e=1
    t27 = 0;
    // #5 op=5 d=0 s1=27 s2=1 e=0
    return t27;

}

