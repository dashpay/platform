package org.dashfoundation.example.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/** Dash brand blue — matches the iOS `AccentColor` asset. */
val DashBlue = Color(0xFF008DE4)
val DashBlueDark = Color(0xFF66BDF2)

// Dynamic color is intentionally OFF: brand parity with the iOS app beats
// Material You wallpaper tinting for an SDK example app.
private val LightColors = lightColorScheme(
    primary = DashBlue,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFD0E8FA),
    onPrimaryContainer = Color(0xFF00293F),
    secondary = Color(0xFF50606E),
    error = Color(0xFFBA1A1A),
)

private val DarkColors = darkColorScheme(
    primary = DashBlueDark,
    onPrimary = Color(0xFF00344A),
    primaryContainer = Color(0xFF004C6E),
    onPrimaryContainer = Color(0xFFD0E8FA),
    secondary = Color(0xFFB8C8D8),
    error = Color(0xFFFFB4AB),
)

@Composable
fun ExampleTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        content = content,
    )
}
