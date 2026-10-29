/* touch-kit C API (touch_kit.dll / libtouch_kit.so). See crates/touch-ffi/src/lib.rs for details and
 * docs/integrating.md for a walkthrough.
 *
 * Everything crosses as UTF-8 JSON. Strings returned by tk_* functions belong to the caller and are
 * freed with tk_free (except tk_version). On failure a function returns NULL/false and tk_last_error
 * gives the reason, on the same thread. */
#ifndef TOUCH_KIT_H
#define TOUCH_KIT_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TkDevice TkDevice;

/* One message (a status, pointer event or gesture) as JSON, valid only during the call. Runs on a
 * library thread, never the caller's UI thread. */
typedef void (*TkCallback)(void *user, const char *json);

/* e.g. "1.2.1"; static, don't free. */
const char *tk_version(void);

/* The last error on this thread, or NULL; taken once. */
char *tk_last_error(void);

void tk_free(char *s);

/* JSON array of {vid, pid, port, serial, product, driver, ready}. */
char *tk_touchscreens(void);

/* A device report; records record_seconds of touches on each ready touchscreen first (0 = none).
 * Blocks. Stop running devices first. */
char *tk_report(uint32_t record_seconds);

/* config_json: {"device": {...}, "panel": {...}, "gestures": {...}, "taken": [ports],
 * "replay": "file.jsonl"}, every key optional; NULL = any touchscreen with defaults. Messages go to
 * callback until tk_stop. */
TkDevice *tk_start(const char *config_json, TkCallback callback, void *user);

/* New panel geometry and gesture thresholds (NULL = defaults) without reopening the device. */
bool tk_reconfigure(TkDevice *device, const char *panel_json, const char *gestures_json);

/* Stops and frees the device; no callback runs after it returns. */
void tk_stop(TkDevice *device);

#ifdef __cplusplus
}
#endif

#endif
