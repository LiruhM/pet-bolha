package com.petbolha

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.graphics.PixelFormat
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import android.util.DisplayMetrics
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.ImageView
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import java.io.File
import java.io.FileOutputStream

class PetOverlayService : Service(), SensorEventListener {

    private var petHandle: Long = 0
    private var windowManager: WindowManager? = null
    private var petView: FrameLayout? = null
    private var petImageView: ImageView? = null
    private var screenWidth = 0
    private var screenHeight = 0
    private var petX = 0f
    private var petY = 0f
    private var targetX = 0f
    private var targetY = 0f
    private var isDragging = false
    private var lastTouchX = 0f
    private var lastTouchY = 0f

    // Sensors
    private var sensorManager: SensorManager? = null
    private var accelerometer: Sensor? = null
    private var proximity: Sensor? = null
    private var lastAccelX = 0f
    private var lastAccelY = 0f
    private var lastAccelZ = 0f

    // Tick loop
    private val handler = Handler(Looper.getMainLooper())
    private var tickJob: Job? = null
    private var wakeLock: PowerManager.WakeLock? = null

    // Config
    private var currentPersonality = "balanced"
    private var petName = "Bolha"
    private var speechEnabled = true
    private var neuralEnabled = true
    private var clickThroughDefault = true

    // Animation
    private var currentAnimation = "idle"
    private var animationFrame = 0
    private val frameDurationMs = 250L // ~4 FPS idle

    // Coroutines
    private val scope = CoroutineScope(Dispatchers.Main)

    override fun onCreate() {
        super.onCreate()
        Log.d("PetBolha", "PetOverlayService onCreate")

        // Wake lock para manter o service vivo
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "PetBolha::WakeLock")
        wakeLock?.acquire()

        // Screen metrics
        val metrics = DisplayMetrics()
        windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager
        windowManager?.defaultDisplay?.getMetrics(metrics)
        screenWidth = metrics.widthPixels
        screenHeight = metrics.heightPixels

        // Posição inicial (centro inferior)
        petX = (screenWidth * 0.5f).toFloat()
        petY = (screenHeight * 0.8f).toFloat()
        targetX = petX
        targetY = petY

        // Load config
        loadConfig()

        // Inicializa pet-core via JNI
        initPetCore()

        // Cria overlay view
        createOverlayView()

        // Registra sensores
        registerSensors()

        // Inicia tick loop
        startTickLoop()

        // Inicia foreground service com notificação
        startForegroundService()
    }

    private fun initPetCore() {
        val configJson = """{
            "personality_id": "$currentPersonality",
            "name": "$petName",
            "auto_start": true,
            "click_through_default": $clickThroughDefault,
            "speech_enabled": $speechEnabled,
            "neural_enabled": $neuralEnabled,
            "idle_threshold_secs": 30,
            "speech_min_interval_secs": 120,
            "window_follow_enabled": true
        }""".trimIndent()

        petHandle = PetNative.petCreate(petName, currentPersonality, configJson)
        Log.d("PetBolha", "Pet created with handle: $petHandle")

        // Tenta carregar estado salvo
        PetNative.petLoad(petHandle)
    }

    private fun createOverlayView() {
        petView = FrameLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.WRAP_CONTENT
            )
        }

        petImageView = ImageView(this).apply {
            // Placeholder - sprites serão carregados dos assets
            setImageResource(android.R.drawable.star_big_on)
            scaleType = ImageView.ScaleType.FIT_CENTER
            layoutParams = FrameLayout.LayoutParams(96, 96) // 64 * 1.5
        }

        petView?.addView(petImageView!!)

        val params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            } else {
                WindowManager.LayoutParams.TYPE_PHONE
            },
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
                or WindowManager.LayoutParams.FLAG_WATCH_OUTSIDE_TOUCH
                or WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
                or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = petX.toInt() - 48
            y = petY.toInt() - 48
        }

        windowManager?.addView(petView, params)

        // Touch listener
        petView?.setOnTouchListener { _, event ->
            onPetTouch(event)
            true
        }
    }

    private fun onPetTouch(event: MotionEvent): Boolean {
        when (event.action) {
            MotionEvent.ACTION_DOWN -> {
                isDragging = true
                lastTouchX = event.rawX
                lastTouchY = event.rawY
                // Interação: Pet (carinho)
                PetNative.petInteract(petHandle, PetNative.UserAction.PET.value)
            }
            MotionEvent.ACTION_MOVE -> {
                if (isDragging) {
                    val dx = event.rawX - lastTouchX
                    val dy = event.rawY - lastTouchY
                    petX += dx
                    petY += dy
                    // Clamp to screen
                    petX = petX.coerceIn(48f, (screenWidth - 48).toFloat())
                    petY = petY.coerceIn(48f, (screenHeight - 48).toFloat())
                    updatePetPosition()
                    lastTouchX = event.rawX
                    lastTouchY = event.rawY
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isDragging = false
                // Long press = Play
                if (event.eventTime - event.downTime > 500) {
                    PetNative.petInteract(petHandle, PetNative.UserAction.PLAY.value)
                }
            }
        }
        return true
    }

    private fun updatePetPosition() {
        petView?.x = petX - 48
        petView?.y = petY - 48
    }

    private fun registerSensors() {
        sensorManager = getSystemService(Context.SENSOR_SERVICE) as SensorManager
        accelerometer = sensorManager?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
        proximity = sensorManager?.getDefaultSensor(Sensor.TYPE_PROXIMITY)

        accelerometer?.let { sensorManager?.registerListener(this, it, SensorManager.SENSOR_DELAY_GAME) }
        proximity?.let { sensorManager?.registerListener(this, it, SensorManager.SENSOR_DELAY_NORMAL) }
    }

    override fun onSensorChanged(event: SensorEvent?) {
        event?.let {
            when (it.sensor.type) {
                Sensor.TYPE_ACCELEROMETER -> {
                    lastAccelX = it.values[0]
                    lastAccelY = it.values[1]
                    lastAccelZ = it.values[2]
                    // Atualiza cursor position no pet-core baseada no acelerômetro
                    // Inverte Y porque eixo do telefone é diferente
                    val cursorX = (0.5f - lastAccelX / 20f).coerceIn(0f, 1f)
                    val cursorY = (0.5f + lastAccelY / 20f).coerceIn(0f, 1f)
                    PetNative.petSetCursor(petHandle, cursorX, cursorY)
                }
                Sensor.TYPE_PROXIMITY -> {
                    // Proximidade = carinho
                    if (it.values[0] < it.sensor.maximumRange) {
                        PetNative.petInteract(petHandle, PetNative.UserAction.PET.value)
                    }
                }
            }
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {}

    private fun startTickLoop() {
        tickJob = scope.launch {
            var lastTime = System.currentTimeMillis()
            while (isActive) {
                val now = System.currentTimeMillis()
                val dtSecs = (now - lastTime) / 1000f
                lastTime = now

                // Tick do pet-core
                PetNative.petTick(petHandle, dtSecs)

                // Verifica se deve renderizar
                if (PetNative.petShouldRender(petHandle)) {
                    updateFromPetState()
                }

                // Verifica fala
                if (speechEnabled) {
                    val speech = PetNative.petGetSpeech(petHandle)
                    speech?.let { showSpeechBubble(it) }
                }

                // Sleep para ~10Hz
                kotlinx.coroutines.delay(100)
            }
        }
    }

    private fun updateFromPetState() {
        val stateJson = PetNative.petGetState(petHandle)
        Log.d("PetBolha", "Pet state: $stateJson")

        // Parse JSON e atualiza view
        // TODO: usar kotlinx.serialization ou org.json
        // Por enquanto, atualiza animação
        val animation = PetNative.petGetAnimation(petHandle)
        if (animation != currentAnimation) {
            currentAnimation = animation
            animationFrame = 0
            loadAnimationFrame(animation, 0)
        } else {
            // Próximo frame
            animationFrame++
            loadAnimationFrame(animation, animationFrame)
        }
    }

    private fun loadAnimationFrame(animation: String, frame: Int) {
        // TODO: carregar sprite do assets/sprites/<personality>.png
        // Por enquanto usa placeholder
        val frameCount = when (animation) {
            "idle", "walk", "sleep" -> 4
            "happy", "play" -> 6
            "angry", "think", "pet" -> 3
            "eat" -> 2
            else -> 4
        }
        val frameIndex = frame % frameCount
        // petImageView?.setImageBitmap(...)
    }

    private fun showSpeechBubble(text: String) {
        Log.d("PetBolha", "Pet says: $text")
        // TODO: mostrar bolha de fala acima do pet
        // Pode usar um TextView no petView ou Toast customizado
    }

    private fun startForegroundService() {
        val channelId = "pet_bolha_channel"
        val notification = NotificationCompat.Builder(this, channelId)
            .setContentTitle("Pet Bolha")
            .setContentText("$petName está ativo")
            .setSmallIcon(android.R.drawable.star_big_on)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setOngoing(true)
            .build()

        NotificationManagerCompat.from(this).notify(1, notification)
        startForeground(1, notification)
    }

    private fun loadConfig() {
        val prefs = getSharedPreferences("pet_bolha_config", MODE_PRIVATE)
        currentPersonality = prefs.getString("personality", "balanced") ?: "balanced"
        petName = prefs.getString("name", "Bolha") ?: "Bolha"
        speechEnabled = prefs.getBoolean("speech_enabled", true)
        neuralEnabled = prefs.getBoolean("neural_enabled", true)
        clickThroughDefault = prefs.getBoolean("click_through", true)
    }

    fun saveConfig() {
        val prefs = getSharedPreferences("pet_bolha_config", MODE_PRIVATE).edit()
            .putString("personality", currentPersonality)
            .putString("name", petName)
            .putBoolean("speech_enabled", speechEnabled)
            .putBoolean("neural_enabled", neuralEnabled)
            .putBoolean("click_through", clickThroughDefault)
            .apply()
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.d("PetBolha", "PetOverlayService onDestroy")

        tickJob?.cancel()
        scope.cancel()

        // Salva estado do pet
        PetNative.petSave(petHandle)
        PetNative.petDestroy(petHandle)

        // Remove overlay
        if (petView != null && windowManager != null) {
            try {
                windowManager?.removeView(petView)
            } catch (e: Exception) {
                Log.e("PetBolha", "Error removing view", e)
            }
        }

        // Unregister sensors
        sensorManager?.unregisterListener(this)

        // Release wake lock
        wakeLock?.release()
        wakeLock = null
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // Métodos para MainActivity chamar
    fun setPersonality(personality: String) {
        currentPersonality = personality
        saveConfig()
        // Recria pet com nova personalidade
        PetNative.petDestroy(petHandle)
        initPetCore()
    }

    fun setPetName(name: String) {
        petName = name
        saveConfig()
    }

    fun setSpeechEnabled(enabled: Boolean) {
        speechEnabled = enabled
        saveConfig()
    }

    fun setNeuralEnabled(enabled: Boolean) {
        neuralEnabled = enabled
        saveConfig()
    }

    fun feedPet() {
        PetNative.petInteract(petHandle, PetNative.UserAction.FEED.value)
    }

    fun putToSleep() {
        PetNative.petInteract(petHandle, PetNative.UserAction.SLEEP.value)
    }

    fun getPetState(): String {
        return PetNative.petGetState(petHandle)
    }

    companion object {
        const val ACTION_FEED = "com.petbolha.FEED"
        const val ACTION_SLEEP = "com.petbolha.SLEEP"
        const val ACTION_CHANGE_PERSONALITY = "com.petbolha.CHANGE_PERSONALITY"
    }

    // Binder para MainActivity
    inner class LocalBinder : android.os.Binder() {
        fun getService(): PetOverlayService = this@PetOverlayService
    }

    private val binder = LocalBinder()

    override fun onBind(intent: Intent?): IBinder? {
        return binder
    }
}