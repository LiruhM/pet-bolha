package com.petbolha

// JNI bindings para pet-core (FFI)
class PetNative {
    companion object {
        init {
            System.loadLibrary("pet_core")
        }
    }

    // Handles opacos
    external fun petCreate(name: String, personalityId: String, configJson: String?): Long
    external fun petDestroy(handle: Long)
    external fun petTick(handle: Long, dtSecs: Float)
    external fun petInteract(handle: Long, action: Int) // 0=Pet, 1=Play, 2=Feed, 3=Sleep, 4=Ignore
    external fun petSetCursor(handle: Long, x: Float, y: Float)
    external fun petSetActiveWindow(handle: Long, windowName: String)
    external fun petGetState(handle: Long): String // JSON
    external fun petGetSpeech(handle: Long): String? // null se não tem fala
    external fun petSetClickThrough(handle: Long, enabled: Boolean)
    external fun petSave(handle: Long): Int
    external fun petLoad(handle: Long): Int
    external fun petShouldRender(handle: Long): Boolean
    external fun petGetAnimation(handle: Long): String

    enum class UserAction(val value: Int) {
        PET(0),
        PLAY(1),
        FEED(2),
        SLEEP(3),
        IGNORE(4)
    }
}