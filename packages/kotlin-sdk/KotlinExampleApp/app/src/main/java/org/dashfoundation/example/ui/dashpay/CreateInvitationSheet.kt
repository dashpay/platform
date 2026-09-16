package org.dashfoundation.example.ui.dashpay

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.PersistableBundle
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.tokens.Dashpay
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.util.generateQrBitmap
import org.dashfoundation.example.util.parseDashToDuffs
import org.dashfoundation.example.util.toHex

/**
 * Create-invitation sheet — port of `CreateInvitationSheet.swift`. Amount
 * entry (default **0.03 DASH**; the help copy renders the Rust-enforced
 * bounds from [Dashpay.minInvitationDuffs] / [Dashpay.maxInvitationDuffs]
 * rather than mirroring them) + the "send a contact request back to me"
 * toggle (drives the
 * optional inviter info; disabled without a DPNS username), then
 * `dashpay.createInvitation` → QR + share + copy.
 *
 * Bearer-secret handling: the link embeds the one-time voucher private key.
 * It is never logged; the QR renders from an in-memory bitmap; sharing is
 * TEXT-only (no secret-bearing temp image file); and the clipboard copy is
 * flagged sensitive on API 33+ **and actively compare-and-cleared after
 * ~60 s** — Android has no local-only or auto-expiring clipboard primitive
 * (unlike iOS `localOnly` + `expirationDate`), so the device-scoping half
 * of iOS's protection is an accepted platform gap.
 *
 * Runs the create in the application scope so a sheet dismissal cannot
 * cancel a broadcast-in-flight funding transaction; the sheet gates its own
 * dismissal while creating (UI single-flight — Rust's per-wallet
 * build-persist mutex is the backstop, not the primary guard).
 */
@Composable
fun CreateInvitationSheet(
    preferredIdentityIdHex: String? = null,
    onBusyChange: (Boolean) -> Unit = {},
    onClose: () -> Unit,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val context = LocalContext.current

    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: kotlinx.coroutines.flow.MutableStateFlow(
            emptyMap<String, org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet>(),
        )
    }.collectAsStateWithLifecycle()

    // Inviter identity — drives the funding wallet AND the opt-in toggle.
    // The DashPay tab's ACTIVE selection wins (passed via the route); only
    // when none was active fall back to the first wallet-owned identity
    // whose wallet is loaded. An identity-less wallet can still create a
    // pure funding voucher.
    val walletOwned by remember(network) {
        container.database.identityDao().observeWalletOwnedByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(emptyList())
    val inviterIdentity = remember(walletOwned, walletsMap, preferredIdentityIdHex) {
        val loaded = walletOwned.filter {
            it.walletId != null && walletsMap.containsKey(it.walletId!!.toHex())
        }
        loaded.firstOrNull { it.identityId.toHex() == preferredIdentityIdHex }
            ?: loaded.firstOrNull()
    }
    val inviterUsername = inviterIdentity?.let { it.mainDpnsName ?: it.dpnsName }

    var amountText by remember { mutableStateOf("0.03") }
    var sendRequestBack by remember { mutableStateOf(true) }
    var isCreating by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var createdLink by remember { mutableStateOf<String?>(null) }
    val didCopy = remember { mutableStateOf(false) }

    // The result stage renders the only shareable copy of the bearer link:
    // block screenshots/recents thumbnails while it is visible, and keep
    // the host's dismissal gate held (busy) until the user presses Done —
    // an accidental scrim-tap would otherwise discard the link (the
    // voucher stays reclaimable, but the share opportunity is gone).
    val setSecureScreen = org.dashfoundation.example.LocalSecureScreen.current
    androidx.compose.runtime.DisposableEffect(createdLink != null) {
        if (createdLink != null) {
            setSecureScreen(true)
            onBusyChange(true)
        }
        onDispose { if (createdLink != null) setSecureScreen(false) }
    }

    fun create() {
        if (isCreating) return
        val amountDuffs = parseDashToDuffs(amountText)
        if (amountDuffs == null || amountDuffs <= 0) {
            errorMessage = "Enter a valid DASH amount."
            return
        }
        val mgr = manager ?: run { errorMessage = "No wallet manager."; return }
        val wallet = inviterIdentity?.walletId?.let { mgr.wallet(forWalletId = it) }
            ?: walletsMap.values.firstOrNull()
            ?: run { errorMessage = "No wallet loaded."; return }
        val withInviter = sendRequestBack && inviterUsername != null
        isCreating = true
        onBusyChange(true)
        errorMessage = null
        // Application scope: the L1 broadcast must not be cancelled by a
        // sheet teardown mid-flight.
        container.applicationScope.launch {
            try {
                val link = wallet.dashpay.createInvitation(
                    amountDuffs = amountDuffs,
                    fundingAccountIndex = 0,
                    inviterIdentityId = if (withInviter) inviterIdentity?.identityId else null,
                    inviterUsername = if (withInviter) inviterUsername else null,
                    coreSignerHandle = mgr.mnemonicResolverHandle,
                )
                createdLink = link
            } catch (t: Throwable) {
                errorMessage = t.message ?: "Creating the invitation failed."
            } finally {
                isCreating = false
                // A successful create keeps the host's dismissal gate held
                // through the link stage (released by Done); a failure
                // releases it so the sheet can be dismissed with the error.
                onBusyChange(createdLink != null)
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp).testTag("dashpay.invite.create"),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        val link = createdLink
        if (link == null) {
            Text("Invite a friend", style = MaterialTheme.typography.titleLarge)
            OutlinedTextField(
                value = amountText,
                onValueChange = { amountText = it },
                label = { Text("Amount (DASH)") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                enabled = !isCreating,
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.create.amount"),
            )
            Text(
                "Funds a one-time voucher your friend uses to register their identity " +
                    "and a username. Between ${bareDash(Dashpay.minInvitationDuffs)} " +
                    "and ${bareDash(Dashpay.maxInvitationDuffs)} DASH.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Send a contact request back to me")
                Switch(
                    checked = sendRequestBack && inviterUsername != null,
                    onCheckedChange = { sendRequestBack = it },
                    enabled = inviterUsername != null && !isCreating,
                    modifier = Modifier.testTag("dashpay.invite.create.sendBack"),
                )
            }
            Text(
                if (inviterUsername != null) {
                    "Your friend will be asked to add $inviterUsername after they register."
                } else {
                    "Register a username to let invitees add you back automatically."
                },
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            errorMessage?.let {
                Text(it, color = MaterialTheme.colorScheme.error)
            }
            Button(
                onClick = { create() },
                enabled = !isCreating && walletsMap.isNotEmpty(),
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.create.submit"),
            ) {
                if (isCreating) {
                    CircularProgressIndicator(Modifier.size(18.dp))
                    Text("  Creating…")
                } else {
                    Text("Create Invitation")
                }
            }
        } else {
            Text("Invitation ready", style = MaterialTheme.typography.titleLarge)
            Text(
                "Share this link with your friend. It funds their new identity — " +
                    "treat it like cash.",
                style = MaterialTheme.typography.bodyMedium,
            )
            remember(link) { generateQrBitmap(link) }?.let { qr ->
                Image(
                    bitmap = qr.asImageBitmap(),
                    contentDescription = "Invitation QR",
                    modifier = Modifier.size(220.dp).align(Alignment.CenterHorizontally),
                )
            }
            Button(
                onClick = { shareInvitationLink(context, link) },
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.create.share"),
            ) { Text("Share link") }
            Button(
                onClick = {
                    val label = copyInvitationLinkSensitive(context, link)
                    didCopy.value = true
                    // Compare-and-clear after ~60 s (iOS parity window):
                    // Android has no clipboard expiry, so clear it ourselves.
                    // Application scope — a composition scope dies with the
                    // sheet, which would leave the bearer link on the
                    // clipboard indefinitely. The per-copy label nonce keeps
                    // the delayed clear from wiping a NEWER copy (this one's
                    // or any other clip that replaced it).
                    val appContext = context.applicationContext
                    container.applicationScope.launch {
                        delay(60_000)
                        if (clearClipboardIfLabelMatches(appContext, label)) {
                            // The link left the clipboard — the button must
                            // stop claiming otherwise. (Writing to dead
                            // composition state is harmless.)
                            didCopy.value = false
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.create.copy"),
            ) { Text(if (didCopy.value) "Copied" else "Copy link") }
            Text(
                "The link contains a one-time key. Anyone who has it can claim the " +
                    "funds, so share it privately.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            TextButton(
                onClick = onClose,
                modifier = Modifier.fillMaxWidth().testTag("dashpay.invite.create.done"),
            ) { Text("Done") }
        }
    }
}

/** Text-only share — never a temp image file carrying the bearer link. */
private fun shareInvitationLink(context: Context, link: String) {
    val send = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(Intent.EXTRA_TEXT, link)
    }
    context.startActivity(Intent.createChooser(send, "Share invitation"))
}

/**
 * Copy the bearer link with a unique per-copy label (returned) so the
 * delayed clear can target exactly this copy, plus the API 33+ sensitive
 * flag (masks the system clipboard preview; earlier APIs have no masking).
 */
private fun copyInvitationLinkSensitive(context: Context, link: String): String {
    val label = "dashpay-invitation-${android.os.SystemClock.elapsedRealtimeNanos()}"
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText(label, link)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        clip.description.extras = PersistableBundle().apply {
            putBoolean(ClipDescription.EXTRA_IS_SENSITIVE, true)
        }
    }
    clipboard.setPrimaryClip(clip)
    return label
}

/**
 * Clear the clipboard only if OUR exact copy (by label nonce) is still
 * current. Returns whether a clear happened.
 */
private fun clearClipboardIfLabelMatches(context: Context, label: String): Boolean {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    if (clipboard.primaryClipDescription?.label != label) return false
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        clipboard.clearPrimaryClip()
    } else {
        clipboard.setPrimaryClip(ClipData.newPlainText("", ""))
    }
    return true
}

/**
 * duffs → a bare DASH decimal ("0.26", not "0.26000000" or "0,26") for the
 * help copy — [java.math.BigDecimal]-backed like [parseDashToDuffs], so it
 * is exact and locale-independent (the amount field parses "." input).
 */
private fun bareDash(duffs: Long): String =
    java.math.BigDecimal(duffs).movePointLeft(8).stripTrailingZeros().toPlainString()
