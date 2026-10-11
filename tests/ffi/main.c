/* Exercises sa.h from C. Usage: sa_ffi_test [SA2 install dir]
 * Without a directory it checks error handling and the Chao save API only.
 * Exits non-zero on the first failure. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sa.h"

#define CHECK(expr)                                                                 \
    do {                                                                            \
        enum SaStatus s_ = (expr);                                                  \
        if (s_ != SA_STATUS_OK) {                                                   \
            fprintf(stderr, "FAIL %s -> %d: %s\n", #expr, (int)s_, sa_last_error()); \
            return 1;                                                               \
        }                                                                           \
    } while (0)

static int check_errors(void) {
    SaGame *g = NULL;
    if (sa_game_open("Z:/no/such/install", &g) != SA_STATUS_NOT_FOUND || g != NULL) {
        fprintf(stderr, "FAIL: missing install should be NOT_FOUND\n");
        return 1;
    }
    if (strlen(sa_last_error()) == 0) {
        fprintf(stderr, "FAIL: no error message\n");
        return 1;
    }
    if (sa_game_open(NULL, &g) != SA_STATUS_NULL_ARGUMENT) {
        fprintf(stderr, "FAIL: NULL path should be NULL_ARGUMENT\n");
        return 1;
    }
    printf("errors: ok (%s)\n", sa_last_error());
    return 0;
}

static int check_chao(void) {
    /* Slot array offset 0xA64 + two 0x800-byte slots. */
    size_t len = 0xA64 + 2 * 0x800;
    uint8_t *buf = calloc(len, 1);
    SaChaoFile *f = NULL;
    SaChao c;
    uint8_t *out = NULL;
    size_t out_len = 0;
    uint32_t slots = 0;
    CHECK(sa_chao_file_read(buf, len, &f));
    CHECK(sa_chao_file_slot_count(f, &slots));
    CHECK(sa_chao_set_type(f, 0, 2 /* Child */));
    CHECK(sa_chao_set_stat(f, 0, 1 /* fly */, 5, 99, 3266));
    CHECK(sa_chao_set_alignment(f, 0, 1.0f));
    CHECK(sa_chao_get(f, 0, &c));
    if (slots != 2 || c.chao_type != 2 || c.stat_points[1] != 3266 || c.alignment != 1.0f) {
        fprintf(stderr, "FAIL: chao fields slots=%u type=%u fly=%u align=%f\n", slots, c.chao_type, c.stat_points[1], c.alignment);
        return 1;
    }
    CHECK(sa_chao_file_write(f, &out, &out_len));
    if (out_len != len || out[0xA64 + 0x80] != 2) {
        fprintf(stderr, "FAIL: written save\n");
        return 1;
    }
    sa_bytes_free(out, out_len);
    sa_chao_file_free(f);
    free(buf);
    printf("chao: ok\n");
    return 0;
}

static int check_game(const char *dir) {
    SaGame *g = NULL;
    SaPhysics p;
    SaMesh *m = NULL;
    SaMeshView v;
    SaSubMesh sm;
    SaTextureList *t = NULL;
    SaTextureView tv;
    SaCollision *col = NULL;
    SaCollisionView cv;
    uint32_t kind = 99, ntex = 0;
    CHECK(sa_game_open(dir, &g));
    CHECK(sa_game_kind(g, &kind));
    CHECK(sa_physics(g, SA_CHARACTER_SONIC, &p));
    printf("sonic physics: hang_time=%d jump_speed=%.3f run_speed=%.3f radius=%.2f height=%.2f\n", p.hang_time, p.jump_speed, p.run_speed, p.radius, p.height);
    if (sa_physics(g, 99, &p) != SA_STATUS_RANGE) {
        fprintf(stderr, "FAIL: character 99 should be RANGE\n");
        return 1;
    }

    CHECK(sa_mesh_load_character(g, SA_CHARACTER_SONIC, SA_COSTUME_DEFAULT, &m));
    CHECK(sa_mesh_view(m, &v));
    CHECK(sa_mesh_submesh(m, 0, &sm));
    CHECK(sa_textures_load_character(g, SA_CHARACTER_SONIC, SA_COSTUME_DEFAULT, &t));
    CHECK(sa_textures_count(t, &ntex));
    CHECK(sa_textures_get(t, 0, &tv));
    printf("sonic mesh: %u vertices, %u triangles, %u submeshes (first texture %d); %u textures, #0 %s %ux%u\n",
           v.vertex_count, v.index_count / 3, v.submesh_count, sm.texture_id, ntex, tv.name, tv.width, tv.height);
    if (v.vertex_count == 0 || v.index_count % 3 != 0 || ntex == 0 || tv.rgba == NULL) {
        fprintf(stderr, "FAIL: sonic mesh/textures\n");
        return 1;
    }
    sa_textures_free(t);
    sa_mesh_free(m);

    CHECK(sa_mesh_load_landtable(g, "objLandTableNeut", &m));
    CHECK(sa_mesh_view(m, &v));
    CHECK(sa_collision_load_landtable(g, "objLandTableNeut", &col));
    CHECK(sa_collision_view(col, &cv));
    printf("neutral garden: %u visible triangles, %u collision triangles\n", v.index_count / 3, cv.triangle_count);
    sa_collision_free(col);
    sa_mesh_free(m);
    sa_game_close(g);
    printf("game (kind %u): ok\n", kind);
    return 0;
}

int main(int argc, char **argv) {
    if (check_errors() || check_chao()) return 1;
    if (argc > 1 && check_game(argv[1])) return 1;
    printf("ALL OK\n");
    return 0;
}
