#include <ctype.h>
#include <stddef.h>
#include <stdint.h>

/* MINUX native C core: model identifier validation and UI easing. */
uint32_t minux_engine_version(void) {
    return 2u;
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
    if (progress < 0.0f) progress = 0.0f;
    if (progress > 1.0f) progress = 1.0f;
    {
        const float inverse = 1.0f - progress;
        return 1.0f - inverse * inverse * inverse;
    }
}
