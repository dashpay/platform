package org.dashfoundation.dashsdk.security;

import static org.junit.Assert.assertArrayEquals;

import org.junit.Test;

/** Java-source compatibility guard for the public encrypted-blob value type. */
public class KeystoreManagerJavaInteropTest {

    @Test
    public void encryptedBlobRetainsItsTwoArgumentConstructor() {
        byte[] iv = new byte[] {1, 2, 3};
        byte[] ciphertext = new byte[] {4, 5, 6};

        KeystoreManager.EncryptedBlob blob =
                new KeystoreManager.EncryptedBlob(iv, ciphertext);

        assertArrayEquals(iv, blob.getIv());
        assertArrayEquals(ciphertext, blob.getCiphertext());
    }
}
