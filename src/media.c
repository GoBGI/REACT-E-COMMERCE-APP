
#include "musicd.h"

static const char *get_metadata(
    const AVFormatContext *avctx,
    int stream_index,
    const char *key
) {