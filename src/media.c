
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
        return NULL;
    }

    AVFormatContext *avctx = NULL;
    if (avformat_open_input(&avctx, path, NULL, NULL) < 0) {
        return NULL;
    }

    if (avctx->nb_streams < 1 || avctx->duration < 1) {
        if (avformat_find_stream_info(avctx, NULL) < 0) {
            avformat_close_input(&avctx);
            return NULL;
        }
    }

    struct MediaInfo *media_info = calloc(1, sizeof(struct MediaInfo));
    memset(media_info, 0, sizeof(struct MediaInfo));

    struct TrackInfo *track_cur = NULL;
    struct ImageInfo *image_cur = NULL;

    // av_dump_format(avctx, 0, NULL, 0);

    for (unsigned int i = 0; i < avctx->nb_streams; ++i) {
        struct TrackInfo *track_info = try_get_track_info(avctx, i, 0, path);
        if (track_info) {
            if (!track_cur) {
                media_info->tracks = track_info;
            } else {
                track_cur->next = track_info;
            }

            track_cur = track_info;

            continue;
        }

        struct ImageInfo *image_info = try_get_image_info(avctx, i, path);
        if (image_info) {
            if (!image_cur) {
                media_info->images = image_info;
            } else {
                image_cur->next = image_info;
            }

            image_cur = image_info;

            continue;
        }
    }

    avformat_close_input(&avctx);
    return media_info;
}

static char *extract_name_from_path(const char *path) {
    const char *start, *end;

    for (start = (char *)path + strlen(path); start > path && *(start - 1) != '/'; --start) { }
    for (end = start; *end != '.' && *end != '\0'; ++end) { }

    return av_strndup(start, end - start);
}

static struct TrackInfo *try_get_track_info(
    const AVFormatContext *avctx,
    int stream_index,
    int track_index,
    const char *path
) {
    const AVStream *stream = avctx->streams[stream_index];

    if (stream->codecpar->codec_type != AVMEDIA_TYPE_AUDIO) {
        return NULL;
    }

    double length = avctx->duration > 0
        ? avctx->duration / (double)AV_TIME_BASE
        : stream->duration * (double)av_q2d(stream->time_base);

    if (length <= 0) {
        return NULL;
    }

    struct TrackInfo *track_info = malloc(sizeof(struct TrackInfo));
    memset(track_info, 0, sizeof(struct TrackInfo));

    track_info->stream_index = stream_index;
    track_info->track_index = track_index;

    track_info->length = length;

    const char *tmp = get_metadata(avctx, stream_index, "track");
    if (tmp) {
        sscanf(tmp, "%d", &track_info->number);