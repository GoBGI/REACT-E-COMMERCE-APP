
#include "musicd.h"

static enum AVSampleFormat find_sample_fmt(
    enum AVSampleFormat src_fmt,
    const enum AVSampleFormat *dst_fmts
) {
    if (!dst_fmts) {
        return src_fmt;
    }

    for (const enum AVSampleFormat *iter = dst_fmts; *iter != -1; ++iter) {
        if (*iter == src_fmt) {
            return src_fmt;
        }
    }

    // Return first supported sample format
    return *dst_fmts;
}

static int find_sample_rate(int sample_rate, const int *sample_rates) {
    if (!sample_rates) {
        return sample_rate;
    }

    int closest = 0;

    for (const int *iter = sample_rates; *iter != 0; ++iter) {
        if (*iter == sample_rate) {
            return sample_rate;
        }

        if (abs(*iter - sample_rate) < abs(closest - sample_rate)) {
            closest = *iter;
        }
    }

    return closest;
}

static int audio_stream_write_callback(void *opaque, uint8_t *buf, int buf_size) {
    struct AudioStream *self = (struct AudioStream *)opaque;
    return self->write_callback(self->write_opaque, buf, buf_size);
}

struct AudioStream *audio_stream_open(const struct AudioStreamOptions *options) {
    int result;
    char args[512];

    struct AudioStream *self = malloc(sizeof(struct AudioStream));
    memset(self, 0, sizeof(struct AudioStream));

    // TODO track index

    result = avformat_open_input(&self->in_ctx, options->path, NULL, NULL);
    if (result < 0) {
        lav_error("avformat_open_input", result);
        goto fail;
    }
    
    result = avformat_find_stream_info(self->in_ctx, NULL);
    if (result < 0) {
        lav_error("avformat_find_stream_info", result);
        goto fail;
    }

    if (self->in_ctx->nb_streams <= (uint32_t)options->stream_index) {
        lav_error("audio stream doesn't exist", 0);
        goto fail;
    }

    self->in_stream = self->in_ctx->streams[options->stream_index];

    self->decoder = avcodec_find_decoder(self->in_stream->codecpar->codec_id);
    if (!self->decoder) {
        lav_error("avcodec_find_decoder", result);
        goto fail;
    }

    self->dec_ctx = avcodec_alloc_context3(self->decoder);

    if (avcodec_parameters_to_context(self->dec_ctx, self->in_stream->codecpar)) {
        lav_error("avcodec_parameters_to_context", result);
        goto fail;