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

// Status accents for state rows (synced / running vs. errors). Not part of the
// Material role set, so they're exposed as plain colors the screens can pull
// from `AppStatusColors`.
data class StatusColors(val success: Color, val warning: Color)

val LightStatusColors = StatusColors(success = Color(0xFF1E8E5A), warning = Color(0xFFB26A00))
val DarkStatusColors = StatusColors(success = Color(0xFF69D3A0), warning = Color(0xFFE7B75B))

/** Theme-aware status accents for use inside composables. */
val appStatusColors: StatusColors
    @Composable get() = if (isSystemInDarkTheme()) DarkStatusColors else LightStatusColors

// Dynamic color is intentionally OFF: brand parity with the iOS app beats
// Material You wallpaper tinting for an SDK example app. The neutrals are
// deliberately cool (blue-grey) rather than the Material baseline's violet
// tint, so surfaces sit under the Dash-blue primary instead of fighting it.
private val LightColors = lightColorScheme(
    primary = DashBlue,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFCFE6FA),
    onPrimaryContainer = Color(0xFF001E30),
    secondary = Color(0xFF4E6070),
    onSecondary = Color.White,
    // Nav-bar selection pill + filter chips — a light Dash-blue tint instead
    // of the default lavender.
    secondaryContainer = Color(0xFFD3E7F7),
    onSecondaryContainer = Color(0xFF0B2C42),
    tertiary = Color(0xFF1E8E5A),
    onTertiary = Color.White,
    error = Color(0xFFBA1A1A),
    onError = Color.White,
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    background = Color(0xFFF4F6F9),
    onBackground = Color(0xFF1A1C1E),
    surface = Color(0xFFFCFDFF),
    onSurface = Color(0xFF1A1C1E),
    surfaceVariant = Color(0xFFDEE3E9),
    onSurfaceVariant = Color(0xFF43474C),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFF1F4F8),
    surfaceContainer = Color(0xFFEBEEF3),
    surfaceContainerHigh = Color(0xFFE5E9EE),
    surfaceContainerHighest = Color(0xFFDFE4EA),
    outline = Color(0xFF73777D),
    outlineVariant = Color(0xFFC3C8CE),
)

private val DarkColors = darkColorScheme(
    primary = DashBlueDark,
    onPrimary = Color(0xFF00344F),
    primaryContainer = Color(0xFF004B70),
    onPrimaryContainer = Color(0xFFCFE6FA),
    secondary = Color(0xFFB6C8D9),
    onSecondary = Color(0xFF21313F),
    secondaryContainer = Color(0xFF0F3A57),
    onSecondaryContainer = Color(0xFFCFE6FA),
    tertiary = Color(0xFF69D3A0),
    onTertiary = Color(0xFF00391F),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF101418),
    onBackground = Color(0xFFE1E3E6),
    surface = Color(0xFF14181C),
    onSurface = Color(0xFFE1E3E6),
    surfaceVariant = Color(0xFF41484D),
    onSurfaceVariant = Color(0xFFC1C7CD),
    surfaceContainerLowest = Color(0xFF0D1114),
    surfaceContainerLow = Color(0xFF1A1E22),
    surfaceContainer = Color(0xFF1E2226),
    surfaceContainerHigh = Color(0xFF282D31),
    surfaceContainerHighest = Color(0xFF33383C),
    outline = Color(0xFF8B9298),
    outlineVariant = Color(0xFF41484D),
)

@Composable
fun ExampleTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = AppTypography,
        shapes = AppShapes,
        content = content,
    )
}
