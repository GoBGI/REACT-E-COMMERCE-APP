
#include "musicd.h"

static const char *get_metadata(
    const AVFormatContext *avctx,
    int stream_index,
    const char *key
) {
    const AVDictionaryEntry *entry = av_dict_get(
        avctx->streams[stream_index]->metadata,
        key, NULL, 0);

    if (!entry) {
        entry = av_dict_get(avctx->metadata, key, NULL, 0);
        if (!entry) {
            return NULL;
        }
    }

    return entry->value;
}

static char *copy_metadata(
    const AVFormatContext *avctx,
    int stream_index,
    const char *key
) {
    return av_strdup(get_metadata(avctx, stream_index, key));
}

static struct TrackInfo *try_get_track_info(
    const AVFormatContext *avctx,
    int stream_index,
    int track_index,
    const char *path);

static struct ImageInfo *try_get_image_info(
    const AVFormatContext *avctx,
    int stream_index,
    const char *path);

struct MediaInfo *media_info_from_path(const char *path) {
    const AVOutputFormat *fmt = av_guess_format(NULL, path, NULL);

    if (!fmt) {
        return NULL;
    }

    if (fmt->audio_codec == AV_CODEC_ID_NONE && fmt->video_codec == AV_CODEC_ID_NONE) {