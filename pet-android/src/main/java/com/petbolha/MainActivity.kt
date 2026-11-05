package com.petbolha

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Bundle
import android.os.IBinder
import android.provider.Settings
import android.util.Log
import android.widget.Button
import android.widget.EditText
import android.widget.RadioGroup
import android.widget.Switch
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {

    private var petService: PetOverlayService? = null
    private var isBound = false

    private val serviceConnection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            val binder = service as PetOverlayService.LocalBinder
            petService = binder.getService()
            isBound = true
            loadCurrentConfig()
        }

        override fun onServiceDisconnected(name: ComponentName) {
            isBound = false
            petService = null
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        // Check overlay permission
        if (!Settings.canDrawOverlays(this)) {
            val intent = Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION)
            startActivity(intent)
        }

        // Bind to service
        val serviceIntent = Intent(this, PetOverlayService::class.java)
        bindService(serviceIntent, serviceConnection, Context.BIND_AUTO_CREATE)

        // UI elements
        val etPetName = findViewById<EditText>(R.id.etPetName)
        val rgPersonality = findViewById<RadioGroup>(R.id.rgPersonality)
        val swSpeech = findViewById<Switch>(R.id.swSpeech)
        val swNeural = findViewById<Switch>(R.id.swNeural)
        val btnFeed = findViewById<Button>(R.id.btnFeed)
        val btnSleep = findViewById<Button>(R.id.btnSleep)
        val btnSave = findViewById<Button>(R.id.btnSave)
        val btnGetState = findViewById<Button>(R.id.btnGetState)

        btnFeed.setOnClickListener {
            petService?.feedPet()
            Toast.makeText(this, "Alimentado!", Toast.LENGTH_SHORT).show()
        }

        btnSleep.setOnClickListener {
            petService?.putToSleep()
            Toast.makeText(this, "Dormindo...", Toast.LENGTH_SHORT).show()
        }

        btnGetState.setOnClickListener {
            val state = petService?.getPetState() ?: "Service não conectado"
            Toast.makeText(this, state, Toast.LENGTH_LONG).show()
            Log.d("PetBolha", "State: $state")
        }

        btnSave.setOnClickListener {
            val name = etPetName.text.toString().trim()
            if (name.isNotEmpty()) {
                petService?.setPetName(name)
            }
            val personality = when (rgPersonality.checkedRadioButtonId) {
                R.id.rbBalanced -> "balanced"
                R.id.rbBryan -> "bryan"
                R.id.rbJorge -> "jorge"
                else -> "balanced"
            }
            petService?.setPersonality(personality)
            petService?.setSpeechEnabled(swSpeech.isChecked)
            petService?.setNeuralEnabled(swNeural.isChecked)
            Toast.makeText(this, "Configurações salvas!", Toast.LENGTH_SHORT).show()
        }
    }

    private fun loadCurrentConfig() {
        val etPetName = findViewById<EditText>(R.id.etPetName)
        val rgPersonality = findViewById<RadioGroup>(R.id.rgPersonality)
        val swSpeech = findViewById<Switch>(R.id.swSpeech)
        val swNeural = findViewById<Switch>(R.id.swNeural)

        petService?.let { service ->
            CoroutineScope(Dispatchers.Main).launch {
                val stateJson = service.getPetState()
                // Parse name from JSON (simplificado)
                // TODO: parse properly
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        if (isBound) {
            unbindService(serviceConnection)
            isBound = false
        }
    }
}