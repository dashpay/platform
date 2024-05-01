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

  // Authentication master key

  const { privateKey: identityMasterPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 0);
  const identityMasterPublicKey = identityMasterPrivateKey.toPublicKey();

  const masterKey = new IdentityPublicKey(1);
  masterKey.setId(0);
  masterKey.setData(identityMasterPublicKey.toBuffer());
  masterKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

  // Authentication high level key

  const { privateKey: identityHighAuthPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 1);
  const identityHighAuthPublicKey = identityHighAuthPrivateKey.toPublicKey();

  const highAuthKey = new IdentityPublicKey(1);
  highAuthKey.setId(1);
  highAuthKey.setData(identityHighAuthPublicKey.toBuffer());
  highAuthKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.HIGH);

  // Authentication critical level key

  const { privateKey: identityCriticalAuthPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 2);
  const identityCriticalAuthPublicKey = identityCriticalAuthPrivateKey.toPublicKey();

  const criticalAuthKey = new IdentityPublicKey(1);
  criticalAuthKey.setId(2);
  criticalAuthKey.setData(identityCriticalAuthPublicKey.toBuffer());
  criticalAuthKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.CRITICAL);

  // Transfer key

  const { privateKey: identityTransferPrivateKey } = account.identities
    .getIdentityHDKeyByIndex(identityIndex, 3);
  const identityTransferPublicKey = identityTransferPrivateKey.toPublicKey();

  const transferKey = new IdentityPublicKey(1);
  transferKey.setId(3);
  transferKey.setPurpose(IdentityPublicKey.PURPOSES.TRANSFER);
  transferKey.setData(identityTransferPublicKey.toBuffer());
  transferKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.CRITICAL);

  // Create Identity
  const identity = dpp.identity.create(
    assetLockProof.createIdentifier(),
    [masterKey, highAuthKey, criticalAuthKey, transferKey],
  );

  // Create ST
  const identityCreateTransition = dpp.identity.createIdentityCreateTransition(
    identity,
    assetLockProof,
  );

  // Create key proofs
  const [
    stMasterKey, stHighAuthKey, stCriticalAuthKey, stTransferKey,
  ] = identityCreateTransition.getPublicKeys();

  // Sign master key

  identityCreateTransition.signByPrivateKey(
    identityMasterPrivateKey.toBuffer(),
    IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  );

  stMasterKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  // Sign high auth key

  identityCreateTransition.signByPrivateKey(
    identityHighAuthPrivateKey.toBuffer(),
    IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  );

  stHighAuthKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  // Sign critical auth key

  identityCreateTransition.signByPrivateKey(
    identityCriticalAuthPrivateKey.toBuffer(),
    IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  );

  stCriticalAuthKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  // Sign transfer key

  identityCreateTransition.signByPrivateKey(
    identityTransferPrivateKey.toBuffer(),
    IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  );

  stTransferKey.setSignature(identityCreateTransition.getSignature());

  identityCreateTransition.setSignature(undefined);

  // Set public keys back after updating their signatures
  identityCreateTransition.setPublicKeys([
    stMasterKey, stHighAuthKey, stCriticalAuthKey, stTransferKey,
  ]);

  // Sign and validate state transition

  identityCreateTransition
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
