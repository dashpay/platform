import { PrivateKey } from '@dashevo/dashcore-lib';
// TODO(wasm): replace with IdentityPublicKey from wasm-dpp
import IdentityPublicKey from '@dashevo/dpp/lib/identity/IdentityPublicKey';
import { Platform } from '../../../Platform';

/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof
 *  for the identity create transition
 * @param {PrivateKey} assetLockPrivateKey - private key used in asset lock
 * @return {{identity: Identity, identityCreateTransition: IdentityCreateTransition}}
 *  - identity, state transition and index of the key used to create it
 * that can be used to sign registration/top-up state transition
 */
export async function createIdentityCreateTransition(
  this : Platform,
  assetLockProof: any,
  assetLockPrivateKey: PrivateKey,
): Promise<{ identity: any, identityCreateTransition: any, identityIndex: number }> {
  const platform = this;
  await platform.initialize();

  const account = await platform.client.getWalletAccount();
  const { wasmDpp } = platform;

  const identityIndex = await account.getUnusedIdentityIndex();

  // @ts-ignore
  const { privateKey: identityMasterPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 0);
  const identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();

  const { privateKey: identitySecondPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 1);
  const identitySecondPublicKey = identitySecondPrivateKey.toPublicKey();

  // Create Identity
  // @ts-ignore
  const identity = wasmDpp.identity.create(
    assetLockProof, [{
      id: 0,
      data: identityMasterPublicKey.toBuffer(),
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    },
    {
      id: 1,
      data: identitySecondPublicKey.toBuffer(),
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      readOnly: false,
    },
    ],
  );

  // Create ST
  const identityCreateTransition = wasmDpp.identity.createIdentityCreateTransition(identity);

  // Create key proofs

  const [masterKey, secondKey] = identityCreateTransition.getPublicKeys();

  await identityCreateTransition
    .signByPrivateKey(identityMasterPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  masterKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature([]);

  await identityCreateTransition
    .signByPrivateKey(identitySecondPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  secondKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature([]);

  // Set public keys back after updating their signatures
  identityCreateTransition.setPublicKeys([masterKey, secondKey]);

  // Sign and validate state transition

  await identityCreateTransition
    .signByPrivateKey(assetLockPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  const result = await wasmDpp.stateTransition.validateBasic(identityCreateTransition);

  if (!result.isValid()) {
    // TODO(wasm): pretty print errors. JSON stringify is not handling wasm errors well
    throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
  }

  return { identity, identityCreateTransition, identityIndex };
}

export default createIdentityCreateTransition;
