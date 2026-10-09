#include <ctype.h>
#include <stddef.h>
#include <stdint.h>

/* Small native core shared by editor search, animation and model-ID checks. */
uint32_t minux_engine_version(void) {
    return 4u;
}

int minux_model_id_is_valid(const uint8_t *data, size_t length) {
    size_t slash = length;
    size_t slash_count = 0u;
    if (data == NULL || length < 3u || length > 256u) return 0;

    for (size_t i = 0u; i < length; ++i) {
        const unsigned char ch = (unsigned char)data[i];
        if (ch == (unsigned char)'/') {
            slash = i;
            ++slash_count;
            continue;
        }
        if (!(isalnum(ch) || ch == (unsigned char)'-' ||
              ch == (unsigned char)'_' || ch == (unsigned char)'.')) return 0;
        if (i > 0u && ch == (unsigned char)'.' &&
            data[i - 1u] == (uint8_t)'.') return 0;
    }

    if (slash_count != 1u || slash == 0u || slash + 1u >= length) return 0;
    if (data[0] == (uint8_t)'.' || data[slash - 1u] == (uint8_t)'.' ||
        data[slash + 1u] == (uint8_t)'.' || data[length - 1u] == (uint8_t)'.') return 0;
    return 1;
}

float minux_ease_out_cubic(float progress) {
    if (!(progress >= 0.0f)) progress = 0.0f;
    if (progress > 1.0f) progress = 1.0f;
    {
        const float inverse = 1.0f - progress;
        return 1.0f - inverse * inverse * inverse;
    }
}

float minux_ease_out_quint(float progress) {
    if (!(progress >= 0.0f)) progress = 0.0f;
    if (progress > 1.0f) progress = 1.0f;
    {
        const float inverse = 1.0f - progress;
        const float inverse2 = inverse * inverse;
        return 1.0f - inverse2 * inverse2 * inverse;
    }
}

float minux_ease_in_out_cubic(float progress) {
    if (!(progress >= 0.0f)) progress = 0.0f;
    if (progress > 1.0f) progress = 1.0f;
    if (progress < 0.5f) return 4.0f * progress * progress * progress;
    {
        const float inverse = -2.0f * progress + 2.0f;
        return 1.0f - (inverse * inverse * inverse) / 2.0f;
    }
}

static int minux_is_word_boundary(const uint8_t *candidate, size_t index) {
    if (index == 0u) return 1;
    const unsigned char previous = (unsigned char)candidate[index - 1u];
    const unsigned char current = (unsigned char)candidate[index];
    if (previous == (unsigned char)'/' || previous == (unsigned char)'\\' ||
        previous == (unsigned char)'_' || previous == (unsigned char)'-' ||
        previous == (unsigned char)'.' || previous == (unsigned char)' ' ||
        previous == (unsigned char)':') return 1;
    if (previous >= (unsigned char)'a' && previous <= (unsigned char)'z' &&
        current >= (unsigned char)'A' && current <= (unsigned char)'Z') return 1;
    if (previous >= (unsigned char)'0' && previous <= (unsigned char)'9' &&
        !(current >= (unsigned char)'0' && current <= (unsigned char)'9')) return 1;
    return 0;
}

static unsigned char minux_ascii_lower(unsigned char value) {
    if (value >= (unsigned char)'A' && value <= (unsigned char)'Z') {
        return (unsigned char)(value + ((unsigned char)'a' - (unsigned char)'A'));
    }
    return value;
}

int32_t minux_search_score(const uint8_t *query, size_t query_len,
                           const uint8_t *candidate, size_t candidate_len) {
    if (query == NULL || candidate == NULL || query_len == 0u ||
        query_len > 256u || candidate_len == 0u || candidate_len > 1024u ||
        query_len > candidate_len) return -1;

    int exact = query_len == candidate_len;
    int prefix = 1;
    for (size_t i = 0u; i < query_len; ++i) {
        const unsigned char q = minux_ascii_lower((unsigned char)query[i]);
        const unsigned char c = minux_ascii_lower((unsigned char)candidate[i]);
        if (q != c) {
            exact = 0;
            prefix = 0;
            break;
        }
    }
    if (exact) return 1000;
    if (prefix) return 800 - (int)(candidate_len - query_len);

    for (size_t start = 1u; start + query_len <= candidate_len; ++start) {
        size_t matched = 0u;
        while (matched < query_len &&
               minux_ascii_lower((unsigned char)query[matched]) ==
               minux_ascii_lower((unsigned char)candidate[start + matched])) ++matched;
        if (matched == query_len) {
            return 600 - (int)(start * 5u) - (int)(candidate_len - query_len);
        }
    }

    size_t matched = 0u;
    size_t gaps = 0u;
    size_t boundaries = 0u;
    size_t previous = 0u;
    for (size_t i = 0u; i < candidate_len && matched < query_len; ++i) {
        if (minux_ascii_lower((unsigned char)query[matched]) ==
            minux_ascii_lower((unsigned char)candidate[i])) {
            if (matched > 0u && i > previous + 1u) gaps += i - previous - 1u;
            if (minux_is_word_boundary(candidate, i)) ++boundaries;
            previous = i;
            ++matched;
        }
    }
    if (matched != query_len) return -1;
    {
        const size_t bounded_bonus = boundaries > 6u ? 6u : boundaries;
        int score = 200 + (int)(bounded_bonus * 8u) -
                    (int)(gaps * 8u) - (int)(candidate_len - query_len);
        return score > 0 ? score : 1;
    }
}
