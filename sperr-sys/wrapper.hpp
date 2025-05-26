#include <stdlib.h>

#include "SPERR_C_API.h"

void free_dst(void * dst) {
    free(dst);
}
