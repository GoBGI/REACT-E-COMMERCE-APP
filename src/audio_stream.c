
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
    }

    if (avcodec_open2(self->dec_ctx, self->decoder, NULL)) {
        lav_error("avcodec_open2", result);
        goto fail;
    }

    if (!self->dec_ctx->channel_layout) {
        self->dec_ctx->channel_layout = av_get_default_channel_layout(self->dec_ctx->channels);
    }

    if (options->start > 0) {
        int64_t seek_pos = options->start / av_q2d(self->in_stream->time_base);
        result = av_seek_frame(self->in_ctx, 0, seek_pos, 0);
        if (result < 0) {
            lav_error("av_seek_frame", result);
            goto fail;
        }
    }

    if (options->length > 0) {
        self->end_pts = (options->start + options->length) / av_q2d(self->in_stream->time_base);
    }

    av_dump_format(self->in_ctx, 0, options->path, 0);

    self->out_ctx = avformat_alloc_context();

    self->out_ctx->oformat = av_guess_format(options->target_codec, NULL, NULL);
    if (!self->out_ctx->oformat) {
        lav_error("av_guess_format", 0);
        goto fail;
    }

    self->out_stream = avformat_new_stream(self->out_ctx, NULL);
    if (!self->out_stream) {
        lav_error("avformat_new_stream", 0);
    }

    // TODO copy metadata

    self->encoder = avcodec_find_encoder(self->out_ctx->oformat->audio_codec);
    if (!self->encoder) {
        lav_error("avcodec_find_encoder", 0);
        goto fail;
    }

    self->enc_ctx = avcodec_alloc_context3(self->encoder);

    self->enc_ctx->sample_fmt = find_sample_fmt(
        self->dec_ctx->sample_fmt,
        self->encoder->sample_fmts);
    self->enc_ctx->sample_rate = find_sample_rate(
        self->dec_ctx->sample_rate,
        self->encoder->supported_samplerates);
    self->enc_ctx->channel_layout = self->dec_ctx->channel_layout;