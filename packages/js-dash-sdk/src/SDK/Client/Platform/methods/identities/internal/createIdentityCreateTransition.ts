import { PrivateKey } from '@dashevo/dashcore-lib';
import { IdentityPublicKey } from '@dashevo/wasm-dpp';
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
  const { dpp } = platform;

  const identityIndex = await account.getUnusedIdentityIndex();

  const { privateKey: identityMasterPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 0);
  const identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();

  const { privateKey: identitySecondPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 1);
  const identitySecondPublicKey = identitySecondPrivateKey.toPublicKey();

  // Create Identity
  const identity = dpp.identity.create(
    assetLockProof.createIdentifier(),
    [new IdentityPublicKey({
      $version: '0',
      id: 0,
      data: identityMasterPublicKey.toBuffer(),
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    }),
    new IdentityPublicKey({
      $version: '0',
      id: 1,
      data: identitySecondPublicKey.toBuffer(),
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      readOnly: false,
    }),
    ],
  );

  // Create ST
  const identityCreateTransition = dpp.identity.createIdentityCreateTransition(
    identity,
    assetLockProof,
  );

  // Create key proofs

  const [masterKey, secondKey] = identityCreateTransition.getPublicKeys();

  await identityCreateTransition
    .signByPrivateKey(identityMasterPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  masterKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  await identityCreateTransition
    .signByPrivateKey(identitySecondPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  secondKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  // Set public keys back after updating their signatures
  identityCreateTransition.setPublicKeys([masterKey, secondKey]);

  // Sign and validate state transition

  await identityCreateTransition
    .signByPrivateKey(assetLockPrivateKey.toBuffer(), IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  // TODO(versioning): restore
  // @ts-ignore
  // const result = await dpp.stateTransition.validateBasic(
  //   identityCreateTransition,
  //   // TODO(v0.24-backport): get rid of this once decided
  //   //  whether we need execution context in wasm bindings
  //   new StateTransitionExecutionContext(),
  // );

  // if (!result.isValid()) {
  //   const messages = result.getErrors().map((error) => error.message);
  //   throw new Error(`StateTransition is invalid - ${JSON.stringify(messages)}`);
  // }

  return { identity, identityCreateTransition, identityIndex };
}

export default createIdentityCreateTransition;
