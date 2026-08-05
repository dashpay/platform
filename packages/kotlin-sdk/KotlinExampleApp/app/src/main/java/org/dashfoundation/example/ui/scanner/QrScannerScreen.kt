package org.dashfoundation.example.ui.scanner

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.navigation.NavHostController
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import org.dashfoundation.example.navigation.QrScanner
import java.net.URLDecoder
import java.nio.charset.StandardCharsets
import java.util.Locale

/**
 * Camera QR scanner — port of `QRScannerView.swift` on CameraX + ML Kit
 * (AVCaptureSession + `.qr` metadata on iOS). Resolves into the same
 * states: scanning (live preview), denied (rationale + Open Settings),
 * checking (permission round-trip). The scanned string returns to the
 * caller via `previousBackStackEntry.savedStateHandle[QrScanner.RESULT_KEY]`
 * exactly once, then the screen pops itself.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QrScannerScreen(navController: NavHostController) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    var hasPermission by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED,
        )
    }
    var permissionRequested by rememberSaveable { mutableStateOf(false) }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        hasPermission = granted
        permissionRequested = true
    }

    // One-shot result delivery guard — ML Kit keeps analyzing frames after
    // the first match; without this the pop could fire twice.
    var delivered by remember { mutableStateOf(false) }
    val appUiState = org.dashfoundation.example.di.LocalAppUiState.current
    fun deliver(raw: String) {
        if (delivered) return
        delivered = true
        // An armed in-memory sink takes the result INSTEAD of the
        // SavedStateHandle: bearer-credential scans (invitation links) must
        // never enter saved-instance state, which Android snapshots to disk.
        val sink = appUiState.scanResultSink
        if (sink != null) {
            appUiState.scanResultSink = null
            sink(raw)
        } else if (isSensitiveInvitationQr(raw)) {
            // The transient sink is lost on process recreation. Classify the
            // content before the generic SavedStateHandle fallback so even a
            // malformed bearer link remains memory-only.
            appUiState.pendingInviteUri.value =
                org.dashfoundation.example.state.SecretInvitationUri(raw)
        } else {
            navController.previousBackStackEntry
                ?.savedStateHandle
                ?.set(QrScanner.RESULT_KEY, raw)
        }
        navController.popBackStack()
    }
    DisposableEffect(Unit) {
        // A cancelled scan must not leave a stale sink armed for a later,
        // unrelated generic scan.
        onDispose { appUiState.scanResultSink = null }
    }

    DisposableEffect(Unit) {
        if (!hasPermission) permissionLauncher.launch(Manifest.permission.CAMERA)
        onDispose { }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Scan QR Code") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Cancel")
                    }
                },
            )
        },
    ) { padding ->
        when {
            hasPermission -> {
                val scanner = remember {
                    BarcodeScanning.getClient(
                        BarcodeScannerOptions.Builder()
                            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
                            .build(),
                    )
                }
                DisposableEffect(Unit) {
                    onDispose { scanner.close() }
                }

                AndroidView(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .testTag("qrScanner.preview"),
                    factory = { viewContext ->
                        val previewView = PreviewView(viewContext)
                        val providerFuture = ProcessCameraProvider.getInstance(viewContext)
                        providerFuture.addListener(
                            {
                                val provider = providerFuture.get()
                                val preview = Preview.Builder().build().also {
                                    it.setSurfaceProvider(previewView.surfaceProvider)
                                }
                                val analysis = ImageAnalysis.Builder()
                                    .setBackpressureStrategy(
                                        ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST,
                                    )
                                    .build()
                                analysis.setAnalyzer(
                                    ContextCompat.getMainExecutor(viewContext),
                                ) { imageProxy ->
                                    analyzeFrame(imageProxy, scanner) { raw -> deliver(raw) }
                                }
                                provider.unbindAll()
                                provider.bindToLifecycle(
                                    lifecycleOwner,
                                    CameraSelector.DEFAULT_BACK_CAMERA,
                                    preview,
                                    analysis,
                                )
                            },
                            ContextCompat.getMainExecutor(viewContext),
                        )
                        previewView
                    },
                    onRelease = {
                        // Unbind so the camera is released the moment the
                        // screen leaves composition (pop-back).
                        runCatching {
                            ProcessCameraProvider.getInstance(context).get().unbindAll()
                        }
                    },
                )
            }

            permissionRequested -> Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Camera Access Needed", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Scanning a payment QR code needs the camera. Grant camera " +
                        "access in the app settings, or go back and paste the " +
                        "address instead.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                Button(
                    onClick = {
                        context.startActivity(
                            Intent(
                                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                                Uri.fromParts("package", context.packageName, null),
                            ),
                        )
                    },
                    modifier = Modifier.testTag("qrScanner.openSettings"),
                ) { Text("Open Settings") }
                TextButton(onClick = { navController.popBackStack() }) { Text("Cancel") }
            }

            else -> Column(
                // Permission round-trip in flight (the `.checking` state).
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    "Requesting camera access…",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * Returns true for any QR payload that may contain a DIP-13 bearer key.
 *
 * Transport matching is deliberately structural rather than parse-validity
 * based: malformed links are still secrets. A `pk` query key is also enough
 * to fail closed, including percent-encoded and mixed-case spellings.
 */
internal fun isSensitiveInvitationQr(raw: String): Boolean {
    val value = raw.trim()
    val lower = value.lowercase(Locale.ROOT)
    val isCustomTransport = Regex("^dashpay://invite(?:[/?#]|$)")
        .containsMatchIn(lower)
    val isLegacyTransport = Regex(
        "^https://invitations\\.dashpay\\.io/applink(?:[/?#]|$)",
    ).containsMatchIn(lower)
    if (isCustomTransport || isLegacyTransport) return true

    val queryStart = value.indexOf('?')
    if (queryStart < 0) return false
    val queryEnd = value.indexOf('#', startIndex = queryStart + 1)
        .takeIf { it >= 0 } ?: value.length
    return value.substring(queryStart + 1, queryEnd)
        .split('&', ';')
        .any { parameter ->
            val rawKey = parameter.substringBefore('=')
            if (rawKey.equals("pk", ignoreCase = true)) {
                true
            } else {
                runCatching {
                    URLDecoder.decode(rawKey, StandardCharsets.UTF_8.name())
                }.getOrNull()?.equals("pk", ignoreCase = true) == true
            }
        }
}

/** Run one camera frame through ML Kit; forward the first QR payload. */
@androidx.annotation.OptIn(androidx.camera.core.ExperimentalGetImage::class)
private fun analyzeFrame(
    imageProxy: ImageProxy,
    scanner: com.google.mlkit.vision.barcode.BarcodeScanner,
    onResult: (String) -> Unit,
) {
    val mediaImage = imageProxy.image
    if (mediaImage == null) {
        imageProxy.close()
        return
    }
    val input = InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
    scanner.process(input)
        .addOnSuccessListener { barcodes ->
            barcodes.firstOrNull { !it.rawValue.isNullOrBlank() }
                ?.rawValue
                ?.let(onResult)
        }
        .addOnCompleteListener { imageProxy.close() }
}
