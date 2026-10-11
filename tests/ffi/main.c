/* Exercises sa.h from C. Usage: sa_ffi_test [SA2 install dir]
 * Without a directory it checks error handling and the Chao save API only.
 * Exits non-zero on the first failure. */
#include <math.h>
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

static int32_t flat_ground(void *user, const float *top, const float *bottom, SaGroundHit *out) {
    (void)user;
    if (top[1] >= 0.0f && bottom[1] <= 0.0f) {
        out->point[0] = top[0]; out->point[1] = 0.0f; out->point[2] = top[2];
        out->normal[0] = 0.0f; out->normal[1] = 1.0f; out->normal[2] = 0.0f;
        out->flags = 1;
        return 1;
    }
    return 0;
}

/* Runs a player for `frames` frames with full stick, then one jump; returns 0 on success. */
static int run_player(const SaPhysics *p, const float start[3], const SaWorld *w, const char *label) {
    SaPlayer *pl = NULL;
    SaPlayerState s;
    SaInput in;
    int f, landed_frames = 0;
    memset(&in, 0, sizeof in);
    CHECK(sa_player_create(p, start, &pl));
    for (f = 0; f < 240; f++) {
        in.stick_angle = f * 0x300;
        in.stick_magnitude = 1.0f;
        in.jump_pressed = (f == 60);
        in.jump_held = (f >= 60 && f < 90);
        CHECK(sa_player_step(pl, &in, w));
        CHECK(sa_player_state(pl, &s));
        landed_frames += s.on_ground;
    }
    printf("%s: ended at (%.1f, %.1f, %.1f), grounded %d/240 frames, action %u\n", label, s.position[0], s.position[1], s.position[2], landed_frames, s.action);
    sa_player_free(pl);
    return landed_frames > 100 ? 0 : 1;
}

/* One enemy 40 units ahead (+X) and 10 up. */
static uint32_t one_target(void *user, const float *from, float *out, uint32_t max) {
    (void)user; (void)from;
    if (max < 1) return 0;
    out[0] = 40.0f; out[1] = 10.0f; out[2] = 0.0f;
    return 1;
}

static int check_homing(const SaPhysics *p) {
    SaWorld *w = NULL;
    SaPlayer *pl = NULL;
    SaPlayerState s;
    SaInput in;
    float start[3] = {0.0f, 0.0f, 0.0f};
    float best = 1e9f;
    int f;
    CHECK(sa_world_from_callbacks(flat_ground, NULL, one_target, NULL, &w));
    CHECK(sa_player_create(p, start, &pl));
    memset(&in, 0, sizeof in);
    in.jump_pressed = 1; in.jump_held = 1;
    CHECK(sa_player_step(pl, &in, w));            /* jump */
    in.jump_pressed = 0;
    for (f = 0; f < 10; f++) CHECK(sa_player_step(pl, &in, w));
    in.jump_pressed = 1;
    CHECK(sa_player_step(pl, &in, w));            /* homing attack */
    CHECK(sa_player_state(pl, &s));
    if (s.action != SA_ACTION_HOMING) {
        fprintf(stderr, "FAIL: expected homing, action %u\n", s.action);
        return 1;
    }
    memset(&in, 0, sizeof in);
    for (f = 0; f < 30; f++) {
        float dx, dy, dz, d;
        CHECK(sa_player_step(pl, &in, w));
        CHECK(sa_player_state(pl, &s));
        dx = s.position[0] - 40.0f; dy = s.position[1] - 10.0f; dz = s.position[2];
        d = dx * dx + dy * dy + dz * dz;
        if (d < best) best = d;
    }
    printf("homing (C target callback): closest approach %.2f units\n", sqrtf(best));
    sa_player_free(pl);
    sa_world_free(w);
    return best < 25.0f ? 0 : 1;
}

static int check_controller(void) {
    SaWorld *w = NULL;
    SaPhysics p;
    float start[3] = {0.0f, 0.0f, 0.0f};
    memset(&p, 0, sizeof p);
    p.hang_time = 60; p.jump_speed = 1.66f; p.jump_add = 0.076f; p.weight = 0.08f;
    p.speed_cap_h = 16.0f; p.speed_cap_v = 16.0f; p.speed_max_h = 3.0f; p.run_accel = 0.05f; p.run_decel = -0.06f;
    p.jog_speed = 0.46f; p.run_speed = 1.39f; p.rush_speed = 2.3f; p.dash_speed = 5.09f; p.height = 10.0f; p.radius = 4.0f;
    CHECK(sa_world_from_callbacks(flat_ground, NULL, NULL, NULL, &w));
    if (run_player(&p, start, w, "controller (C callback world)")) return 1;
    sa_world_free(w);
    return check_homing(&p);
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
    {
        /* Drop Sonic above the floor near the garden centre and run him around. */
        SaWorld *w = NULL;
        float start[3] = {-127.8f, 60.0f, -118.1f};
        CHECK(sa_world_from_collision(col, &w));
        if (run_player(&p, start, w, "sonic in the neutral garden")) return 1;
        sa_world_free(w);
    }
    sa_collision_free(col);
    sa_mesh_free(m);
    sa_game_close(g);
    printf("game (kind %u): ok\n", kind);
    return 0;
}

int main(int argc, char **argv) {
    if (check_errors() || check_chao() || check_controller()) return 1;
    if (argc > 1 && check_game(argv[1])) return 1;
    printf("ALL OK\n");
    return 0;
}
