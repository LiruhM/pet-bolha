#ifndef PET_CORE_H
#define PET_CORE_H

#include <stdint.h>
#include <stddef.h>

// Opaque handle
typedef struct PetHandle PetHandle;
typedef PetHandle* pet_handle_t;

// C FFI exports (matching pet-core/src/ffi.rs)

#ifdef __cplusplus
extern "C" {
#endif

// Creates a new pet
// name: UTF-8 string
// personality_id: UTF-8 string (e.g., "balanced", "bryan", "jorge")
// config_json: UTF-8 JSON string or NULL for defaults
// Returns: opaque handle (0 on failure)
pet_handle_t pet_create(const char* name, const char* personality_id, const char* config_json);

// Destroys a pet
void pet_destroy(pet_handle_t handle);

// Main tick (call periodically when awake)
// dt_secs: delta time in seconds
void pet_tick(pet_handle_t handle, float dt_secs);

// User interaction
// action: 0=Pet, 1=Play, 2=Feed, 3=Sleep, 4=Ignore
void pet_interact(pet_handle_t handle, int action);

// Update cursor position (0.0 to 1.0)
void pet_set_cursor(pet_handle_t handle, float x, float y);

// Update active window name (hashed internally)
void pet_set_active_window(pet_handle_t handle, const char* window_name);

// Get current state as JSON string (caller must free with free())
char* pet_get_state(pet_handle_t handle);

// Get next speech text (returns NULL if none, caller must free)
char* pet_get_speech(pet_handle_t handle);

// Set click-through mode
void pet_set_click_through(pet_handle_t handle, int enabled);

// Save state (neural weights, etc.) - returns 0 on success
int pet_save(pet_handle_t handle);

// Load state - returns 0 on success
int pet_load(pet_handle_t handle);

// Check if pet should render (awake)
int pet_should_render(pet_handle_t handle);

// Get current animation name (caller must free)
char* pet_get_animation(pet_handle_t handle);

#ifdef __cplusplus
}
#endif

#endif // PET_CORE_H