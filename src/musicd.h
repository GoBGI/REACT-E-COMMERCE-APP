#pragma once

#include <stdint.h>
#include <pthread.h>

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavfilter/avfilter.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavutil/opt.h>

struct MediaInfo {
    struct TrackInfo *tracks;
    struct ImageInfo *images;
};

struct TrackInfo {
    struct TrackInfo *next;
    int32_t stream_index;
    int32_t track_index;
    int32_t number;
    char *title;
    char *artist;
    char *album;
    char *album_artist;
    double start;
    double length;
};

struct ImageInfo {
    struct ImageInfo *next;
    int32_t stream_index;
    char *description;
    int32_t width;
    int32_t height;
};

struct AudioStreamOptions {
    char *path;
    int32_t stream_index;
    int32_t track_index;
    double start;
    double length;
    char *target_codec;
};

struct AudioStream {
    AVFormatContext *in_ctx, *out_ctx;
    AVStream *in_stream, *out_stream;
    AVCodec *decoder, *encoder;
    AVCodecContext *dec_ctx, *enc_ctx;
    AVIOContext *out_ioctx;
    AVFilterGraph *filter_graph;
    AVFilterContext *abuffer_ctx, *aformat_ctx, *abuffersink_ctx;
    int64_t end_pts;
    int started;
    int finished;
    void *write_opaque;
    int (*write_callback)(void *opaque, uint8_t *buf, int len);
};

enum LogLevel {
    LogLevelError = 1,
    LogLevelWarn = 2,
    LogLevelInfo = 3,
    LogLevelDebug = 4,
    LogLevelTrace = 5
};

void musicd_log_setup(void (*callback)(int level, const char *));

void lav_error(const char *