
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