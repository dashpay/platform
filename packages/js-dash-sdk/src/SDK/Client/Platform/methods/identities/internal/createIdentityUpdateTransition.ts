import { PrivateKey } from "@dashevo/dashcore-lib";
import { Platform } from "../../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} platform
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof for the identity create transition
 * @param {identity} Identity
 * @param {addPublicKeys} IdentityPublicKey[] | undefined
 * @param {disablePublicKeys} number[] | undefined
 * @param {publicKeysDisabledAt} number | undefined
 * @return {{identity: Identity, identityUpdateTransition: IdentityUpdateTransition}}
 */
export default async function createIdentityUpdateTransition(
  platform : Platform,
  assetLockPrivateKey: PrivateKey,
  identity: any,
  addPublicKeys: IdentityPublicKey[] | undefined,
  disablePublicKeys: number[] | undefined,
  publicKeysDisabledAt: number | undefined,
): Promise<any> {
  await platform.initialize();

  const { dpp } = platform;

  // @ts-ignore
  const identityUpdateTransition = dpp.identity.createIdentityUpdateTransition(
    identity.getId(),
    identity.getRevision() + 1,
    addPublicKeys,
    disablePublicKeys,
    publicKeysDisabledAt,
  );

  await identityUpdateTransition.signByPrivateKey(assetLockPrivateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  const result = await dpp.stateTransition.validateBasic(identityUpdateTransition);

  if (!result.isValid()) {
    throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
  }

  return identityUpdateTransition;
}
