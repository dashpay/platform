import { PrivateKey } from "@dashevo/dashcore-lib";
import { Platform } from "../../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof for the identity create transition
 * @param {PrivateKey} assetLockPrivateKey - private key used in asset lock
 * @return {{identity: Identity, identityCreateTransition: IdentityCreateTransition}} - identity, state transition and index of the key used to create it
 * that can be used to sign registration/top-up state transition
 */
export async function createIdentityCreateTransition(this : Platform, assetLockProof: any, assetLockPrivateKey: PrivateKey): Promise<{ identity: any, identityCreateTransition: any, identityIndex: number }> {
    const platform = this;
    await platform.initialize();

    const account = await platform.client.getWalletAccount();
    const { dpp } = platform;

    const identityIndex = await account.getUnusedIdentityIndex();

    // @ts-ignore
    const { privateKey: identityMasterPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityIndex, 0);
    const identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();

    const { privateKey: identitySecondPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityIndex, 1);
    const identitySecondPublicKey = identitySecondPrivateKey.toPublicKey();

    // Create Identity
    // @ts-ignore
    const identity = dpp.identity.create(
        assetLockProof, [{
          key: identityMasterPublicKey,
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER
        },
        {
          key: identitySecondPublicKey,
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH
      }
      ]
    );

     // Create ST
    const identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);

    // Create key proofs

    const [masterKey, secondKey] = identityCreateTransition.getPublicKeys();

    await identityCreateTransition.signByPrivateKey(identityMasterPrivateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    masterKey.setSignature(identityCreateTransition.getSignature());

    identityCreateTransition.setSignature(undefined);

  await identityCreateTransition.signByPrivateKey(identitySecondPrivateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  secondKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);


  // Sign and validate state transition

    await identityCreateTransition.signByPrivateKey(assetLockPrivateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    const result = await dpp.stateTransition.validateBasic(identityCreateTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    return { identity, identityCreateTransition, identityIndex };
}

export default createIdentityCreateTransition;
