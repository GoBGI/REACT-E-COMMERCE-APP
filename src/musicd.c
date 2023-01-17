#include "musicd.h"

static void (*log_callback)(int level, const char *);

static void lav_callbac