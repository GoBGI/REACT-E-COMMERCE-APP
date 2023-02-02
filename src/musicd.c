#include "musicd.h"

static void (*log_callback)(int level, const char *);

static void lav_callback(void *av_class, int av_level, const char *fmt, va_list va_args) {
    (void)av_class;

    int level = 0;

    if (av_level >= AV_LOG_DEBUG) {
        return;
    } else if (av_level >= AV_LOG_VERBOSE) {
        level = LogLevelTrace;
    } else if (av_level >= AV_LOG_INFO) {
        level = LogLevelDebug;
    } else if (av_level >= AV_LOG_WARNING) {
        level = LogLevelWarn;
    } else {
        level = 