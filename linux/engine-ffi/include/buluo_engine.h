#ifndef BULUO_ENGINE_H
#define BULUO_ENGINE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BuluoEngine BuluoEngine;

enum {
    BULUO_KEY_CHAR = 1,
    BULUO_KEY_SPACE = 2,
    BULUO_KEY_BACKSPACE = 3,
    BULUO_KEY_ENTER = 4,
    BULUO_KEY_ESCAPE = 5,
    BULUO_KEY_PAGE_NEXT = 6,
    BULUO_KEY_PAGE_PREV = 7,
    BULUO_KEY_DIGIT = 8,
    BULUO_KEY_PUNCT = 9,
    BULUO_KEY_SEPARATOR = 10,
    BULUO_KEY_TOGGLE_ASCII = 11
};

typedef struct BuluoOutput {
    int consumed;
    int ascii_mode;
    char *commit;
    char *preedit;
    char **candidates;
    int candidate_count;
    int page;
    int page_count;
} BuluoOutput;

/** dict_path / support_dir may be NULL to use defaults (builtin lexicon + XDG/mac support dir). */
BuluoEngine *buluo_engine_new(const char *dict_path, const char *support_dir);
void buluo_engine_free(BuluoEngine *engine);
void buluo_handle_key(BuluoEngine *engine, int kind, uint32_t ch, BuluoOutput *out);
void buluo_output_free(BuluoOutput *out);

#ifdef __cplusplus
}
#endif

#endif
