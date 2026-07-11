import java.util.*;
import org.bitcoinj.crypto.*;

/**
 * Wire-compat vector generator for the Kotlin-SDK txMetadata migration.
 * Reproduces the legacy org.dashj.platform / dashj-core createTxMetadata
 * key derivation + AES-256-CBC envelope for a given identity slot.
 *
 * Args: <identityIndex> <keyId> <encryptionKeyIndex>
 * (blockchainIdentityECDSADerivationPath = m/9'/1'/5'/0'/<keyType=0'>/<identityIndex'>)
 */
public class LegacyKeyN {
    static String hex(byte[] b){ StringBuilder s=new StringBuilder(); for(byte x:b) s.append(String.format("%02x",x)); return s.toString(); }
    public static void main(String[] a) throws Exception {
        int identityIndex = a.length > 0 ? Integer.parseInt(a[0]) : 0;
        int keyId = a.length > 1 ? Integer.parseInt(a[1]) : 2;
        int encryptionKeyIndex = a.length > 2 ? Integer.parseInt(a[2]) : 1;

        List<String> words = Arrays.asList(
            "abandon","abandon","abandon","abandon","abandon","abandon",
            "abandon","abandon","abandon","abandon","abandon","about");
        byte[] seed = MnemonicCode.toSeed(words, "");

        DeterministicKey root = HDKeyDerivation.createMasterPrivateKey(seed);
        DeterministicHierarchy h = new DeterministicHierarchy(root);

        // blockchainIdentityECDSADerivationPath(testnet) for identity `identityIndex`:
        //   FEATURE_PURPOSE=9', coinType(testnet)=1', FEATURE_PURPOSE_IDENTITIES=5',
        //   0' (subfeature), 0' (keyType=ECDSA), identityIndex'
        List<ChildNumber> accountPath = new ArrayList<>();
        accountPath.add(new ChildNumber(9, true));
        accountPath.add(new ChildNumber(1, true));
        accountPath.add(new ChildNumber(5, true));
        accountPath.add(new ChildNumber(0, true));
        accountPath.add(new ChildNumber(0, true));            // keyType = ECDSA = 0
        accountPath.add(new ChildNumber(identityIndex, true)); // identity index

        int txMetaChild = 32769;       // TxMetadataDocument.childNumber

        List<ChildNumber> full = new ArrayList<>(accountPath);
        full.add(new ChildNumber(keyId, true));
        full.add(new ChildNumber(txMetaChild, true));
        full.add(new ChildNumber(encryptionKeyIndex, true));

        System.out.print("fullPath=m");
        for (ChildNumber c : full) System.out.print("/" + c);
        System.out.println();

        DeterministicKey key = h.get(full, false, true);
        byte[] aesKeyBytes = key.getPrivKeyBytes();
        System.out.println("AES_KEY=" + hex(aesKeyBytes));

        org.bitcoinj.core.ECKey ecKey = org.bitcoinj.core.ECKey.fromPrivate(aesKeyBytes);
        org.bitcoinj.crypto.KeyCrypterAESCBC kc = new org.bitcoinj.crypto.KeyCrypterAESCBC();
        org.bouncycastle.crypto.params.KeyParameter aesKp = kc.deriveKey(ecKey);
        byte[] plaintext = "legacy-txmetadata-wire-compat-vector".getBytes("UTF-8");
        org.bitcoinj.crypto.EncryptedData ed = kc.encrypt(plaintext, aesKp);
        int version = 1; // VERSION_PROTOBUF
        byte[] blob = new byte[1 + ed.initialisationVector.length + ed.encryptedBytes.length];
        blob[0] = (byte) version;
        System.arraycopy(ed.initialisationVector, 0, blob, 1, ed.initialisationVector.length);
        System.arraycopy(ed.encryptedBytes, 0, blob, 1 + ed.initialisationVector.length, ed.encryptedBytes.length);
        System.out.println("PLAINTEXT_hex=" + hex(plaintext));
        System.out.println("BLOB=" + hex(blob));
    }
}
