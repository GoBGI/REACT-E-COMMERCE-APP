#include "musicd.h"

static void (*log_callback)(int level, const char *);

static void lav_callback(void *av_class, int av_level, const char *fmt, va_list va_args) {
    (void)av_class;

    int level = 