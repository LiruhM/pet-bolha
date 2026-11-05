#include <jni.h>
#include <string>
#include <android/log.h>
#include "pet_core.h"  // Header gerado pelo cbindgen ou manual

#define LOG_TAG "PetBolhaJNI"
#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// Helper para converter jstring para std::string
static std::string jstringToString(JNIEnv* env, jstring jstr) {
    if (!jstr) return "";
    const char* chars = env->GetStringUTFChars(jstr, nullptr);
    std::string result(chars);
    env->ReleaseStringUTFChars(jstr, chars);
    return result;
}

// Helper para converter std::string para jstring
static jstring stringToJstring(JNIEnv* env, const std::string& str) {
    return env->NewStringUTF(str.c_str());
}

extern "C" {

// pet_create
JNIEXPORT jlong JNICALL
Java_com_petbolha_PetNative_petCreate(JNIEnv* env, jclass clazz,
                                      jstring name, jstring personalityId, jstring configJson) {
    const char* nameStr = env->GetStringUTFChars(name, nullptr);
    const char* personalityStr = env->GetStringUTFChars(personalityId, nullptr);
    const char* configStr = configJson ? env->GetStringUTFChars(configJson, nullptr) : nullptr;

    pet_handle_t handle = pet_create(nameStr, personalityStr, configStr);

    env->ReleaseStringUTFChars(name, nameStr);
    env->ReleaseStringUTFChars(personalityId, personalityStr);
    if (configStr) env->ReleaseStringUTFChars(configJson, configStr);

    return (jlong)handle;
}

// pet_destroy
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petDestroy(JNIEnv* env, jclass clazz, jlong handle) {
    pet_destroy((pet_handle_t)handle);
}

// pet_tick
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petTick(JNIEnv* env, jclass clazz, jlong handle, jfloat dtSecs) {
    pet_tick((pet_handle_t)handle, dtSecs);
}

// pet_interact
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petInteract(JNIEnv* env, jclass clazz, jlong handle, jint action) {
    pet_interact((pet_handle_t)handle, (int)action);
}

// pet_set_cursor
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petSetCursor(JNIEnv* env, jclass clazz, jlong handle, jfloat x, jfloat y) {
    pet_set_cursor((pet_handle_t)handle, x, y);
}

// pet_set_active_window
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petSetActiveWindow(JNIEnv* env, jclass clazz, jlong handle, jstring windowName) {
    std::string name = jstringToString(env, windowName);
    pet_set_active_window((pet_handle_t)handle, name.c_str());
}

// pet_get_state
JNIEXPORT jstring JNICALL
Java_com_petbolha_PetNative_petGetState(JNIEnv* env, jclass clazz, jlong handle) {
    char* json = pet_get_state((pet_handle_t)handle);
    jstring result = stringToJstring(env, json ? json : "{}");
    if (json) free(json);
    return result;
}

// pet_get_speech
JNIEXPORT jstring JNICALL
Java_com_petbolha_PetNative_petGetSpeech(JNIEnv* env, jclass clazz, jlong handle) {
    char* text = pet_get_speech((pet_handle_t)handle);
    jstring result = text ? stringToJstring(env, text) : nullptr;
    if (text) free(text);
    return result;
}

// pet_set_click_through
JNIEXPORT void JNICALL
Java_com_petbolha_PetNative_petSetClickThrough(JNIEnv* env, jclass clazz, jlong handle, jboolean enabled) {
    pet_set_click_through((pet_handle_t)handle, enabled);
}

// pet_save
JNIEXPORT jint JNICALL
Java_com_petbolha_PetNative_petSave(JNIEnv* env, jclass clazz, jlong handle) {
    return pet_save((pet_handle_t)handle);
}

// pet_load
JNIEXPORT jint JNICALL
Java_com_petbolha_PetNative_petLoad(JNIEnv* env, jclass clazz, jlong handle) {
    return pet_load((pet_handle_t)handle);
}

// pet_should_render
JNIEXPORT jboolean JNICALL
Java_com_petbolha_PetNative_petShouldRender(JNIEnv* env, jclass clazz, jlong handle) {
    return pet_should_render((pet_handle_t)handle);
}

// pet_get_animation
JNIEXPORT jstring JNICALL
Java_com_petbolha_PetNative_petGetAnimation(JNIEnv* env, jclass clazz, jlong handle) {
    char* anim = pet_get_animation((pet_handle_t)handle);
    jstring result = anim ? stringToJstring(env, anim) : stringToJstring(env, "idle");
    if (anim) free(anim);
    return result;
}

} // extern "C"