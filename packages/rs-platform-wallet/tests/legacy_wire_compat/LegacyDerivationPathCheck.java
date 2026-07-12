import java.util.*;
import org.bitcoinj.crypto.ChildNumber;
import org.bitcoinj.params.TestNet3Params;
import org.bitcoinj.wallet.DerivationPathFactory;

/**
 * Provenance verifier for the txMetadata wire-compat vectors.
 *
 * `LegacyKeyN.java` HAND-BUILDS its account path and only asserts, in prose,
 * that at identityIndex 0 that path equals the real dashj factory's output.
 * This tool makes that assertion INDEPENDENTLY REPRODUCIBLE: it drives the
 * REAL `org.bitcoinj.wallet.DerivationPathFactory` (the same class the legacy
 * dash-sdk-kotlin identity-key chain uses) and compares its output to the
 * hand-built path, so a maintainer can confirm the wire-compat anchor without
 * trusting either this repo's prose or an AI agent's word (dashpay/platform#4091,
 * findings 989be307db0f / dd246b5e17d0 / 4c0754158cc6).
 *
 * Empirically (dashj-core 22.0.3, Testnet):
 *   noArg  blockchainIdentityECDSADerivationPath()   = m/9'/1'/5'/0'/0'/0'      (6 components)
 *   int(i) blockchainIdentityECDSADerivationPath(i)  = m/9'/1'/5'/0'/0'/0'/i'   (7 components)
 *
 * The legacy `createTxMetadata` flow derives against the PRIMARY identity — the
 * NO-ARG method — so the legacy tx-metadata key path is
 * `noArg / keyId' / 32769' / encryptionKeyIndex'`, and identityIndex 0 is the
 * only slot a legacy wallet ever wrote. At identityIndex 0 the hand-built path
 * `m/9'/1'/5'/0'/0'/0'` equals `noArg` exactly (`WIRE_COMPAT_ANCHOR_OK=true`
 * below) — that is what makes `legacy_dashj_wire_compat_vector` a genuine
 * anchor.
 *
 * Note the factory's INDEXED overload `int(i)` is a DIFFERENT SHAPE from
 * LegacyKeyN's hand-built nonzero path `m/9'/1'/5'/0'/0'/i'` (the factory keeps
 * the primary-identity `0'` and appends `i'`; LegacyKeyN overwrites the last
 * component). They are printed side by side so it is obvious the nonzero
 * LegacyKeyN vector is NOT a factory-verified legacy sample — it is only the
 * self-referential internal cross-check that
 * `nonzero_identity_index_derivation_slot_is_internally_consistent` documents.
 *
 * Args: [identityIndex]  (default 0)
 */
public class LegacyDerivationPathCheck {
    static String p(List<ChildNumber> l) {
        StringBuilder s = new StringBuilder("m");
        for (ChildNumber c : l) s.append("/").append(c);
        return s.toString();
    }

    static List<ChildNumber> handBuilt(int identityIndex) {
        // Byte-for-byte the account path LegacyKeyN.java constructs.
        return new ArrayList<>(Arrays.asList(
            new ChildNumber(9, true),
            new ChildNumber(1, true),               // coinType = Testnet
            new ChildNumber(5, true),               // FEATURE_PURPOSE_IDENTITIES
            new ChildNumber(0, true),               // subfeature
            new ChildNumber(0, true),               // keyType = ECDSA = 0
            new ChildNumber(identityIndex, true))); // identity index
    }

    public static void main(String[] a) {
        int identityIndex = a.length > 0 ? Integer.parseInt(a[0]) : 0;
        DerivationPathFactory f = DerivationPathFactory.get(TestNet3Params.get());

        List<ChildNumber> noArg = f.blockchainIdentityECDSADerivationPath();
        List<ChildNumber> indexed = f.blockchainIdentityECDSADerivationPath(identityIndex);
        List<ChildNumber> hand = handBuilt(identityIndex);

        System.out.println("identityIndex          = " + identityIndex);
        System.out.println("factory noArg()        = " + p(noArg));
        System.out.println("factory int(index)     = " + p(indexed));
        System.out.println("LegacyKeyN hand-built  = " + p(hand));
        // The load-bearing check: the wire-compat anchor is the PRIMARY-identity
        // (no-arg) path, and LegacyKeyN reproduces it exactly at index 0.
        boolean anchorOk = noArg.equals(handBuilt(0));
        System.out.println("WIRE_COMPAT_ANCHOR_OK  = " + anchorOk
            + "   (noArg factory == LegacyKeyN hand-built at identityIndex 0)");
        if (!anchorOk) {
            System.err.println("PROVENANCE MISMATCH: the wire-compat anchor no longer holds");
            System.exit(1);
        }
    }
}
