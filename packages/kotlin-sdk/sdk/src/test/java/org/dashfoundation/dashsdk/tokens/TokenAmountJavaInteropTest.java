package org.dashfoundation.dashsdk.tokens;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.math.BigInteger;
import java.util.Arrays;
import java.util.Set;
import java.util.stream.Collectors;
import kotlin.coroutines.Continuation;
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet;
import org.junit.Test;

/** Java-source compile guard for the unmangled full-u64 token adapter. */
public class TokenAmountJavaInteropTest {

    @Test
    public void fullDomainConversionIsCallableFromJava() {
        BigInteger max = new BigInteger("18446744073709551615");
        assertEquals(-1L, TokenAmountInterop.toRawLongBits(max));
        assertEquals(max, TokenAmountInterop.fromRawLongBits(-1L));
    }

    @Test
    public void everyAmountBearingActionHasAnUnmangledJavaMethod() {
        Set<String> names = Arrays.stream(JavaTokenActions.class.getDeclaredMethods())
                .map(method -> method.getName())
                .collect(Collectors.toSet());
        assertTrue(names.containsAll(Arrays.asList(
                "mint", "burn", "transfer", "setPrice", "purchase", "updateMaxSupply")));
        assertTrue(Arrays.stream(JavaTokenActions.class.getDeclaredMethods())
                .filter(method -> method.getName().equals("mint"))
                .flatMap(method -> Arrays.stream(method.getParameterTypes()))
                .anyMatch(type -> type.equals(BigInteger.class)));
    }

    /**
     * Java-source compile guard for the real wallet → Tokens → adapter path.
     * It is intentionally not invoked: execution needs native handles and is
     * covered by the device/JNI smoke gate.
     */
    @SuppressWarnings("unused")
    private static Object compileWalletAdapterCall(
            ManagedPlatformWallet wallet,
            Continuation<? super String> continuation) {
        return wallet.getTokens().javaAmounts().mint(
                new byte[32],
                new byte[32],
                0,
                BigInteger.ONE.shiftLeft(63),
                null,
                null,
                GroupAction.None.INSTANCE,
                0,
                1L,
                continuation);
    }
}
