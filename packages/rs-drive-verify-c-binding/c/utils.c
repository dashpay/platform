//
// Created by anton on 05.10.2021.
//

#include <stdlib.h>

char *bin2hex(unsigned char *p, int len)
{
    char *hex = malloc(((2*len) + 1));
    char *r = hex;

    while(len && p)
    {
        (*r) = ((*p) & 0xF0) >> 4;
        (*r) = ((*r) <= 9 ? '0' + (*r) : 'a' - 10 + (*r));
        r++;
        (*r) = ((*p) & 0x0F);
        (*r) = ((*r) <= 9 ? '0' + (*r) : 'a' - 10 + (*r));
        r++;
        p++;
        len--;
    }
    *r = '\0';

    return hex;
}

unsigned char *hex2bin(const char *str)
{
    int len, h;
    unsigned char *result, *err, *p, c;

    err = malloc(1);
    *err = 0;

    if (!str)
        return err;

    if (!*str)
        return err;

    len = 0;
    p = (unsigned char*) str;
    while (*p++)
        len++;

    result = malloc((len/2)+1);
    h = !(len%2) * 4;
    p = result;
    *p = 0;

    c = *str;
    while(c)
    {
        if(('0' <= c) && (c <= '9'))
            *p += (c - '0') << h;
        else if(('A' <= c) && (c <= 'F'))
            *p += (c - 'A' + 10) << h;
        else if(('a' <= c) && (c <= 'f'))
            *p += (c - 'a' + 10) << h;
        else
            return err;

        str++;
        c = *str;

        if (h)
            h = 0;
        else
        {
            h = 4;
            p++;
            *p = 0;
        }
    }

    return result;
}

bool is_array_equal(uint8_t a[], uint8_t b[], int size) {
    for (int i = 0; i < size; i++) {
        if (a[i] != b[i]) {
            return false;
        }
    }
    return true;
}
