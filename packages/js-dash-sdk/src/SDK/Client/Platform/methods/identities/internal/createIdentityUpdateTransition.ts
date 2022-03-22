import { PrivateKey } from "@dashevo/dashcore-lib";
import { Platform } from "../../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"
import Identity from "@dashevo/dpp/lib/identity/Identity";

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} platform
 * @param {PrivateKey} privateKey - private key to sign
 * @param {Identity} identity - identity to update
 * @param {IdentityPublicKey[]} addPublicKeys - public keys to add
 * @param {number[]} disablePublicKeys - public key IDs to disable
 * @param {number} publicKeysDisabledAt - timestamp to disable at
 * @return {{identity: Identity, identityUpdateTransition: IdentityUpdateTransition}}
 */
export default async function createIdentityUpdateTransition(
  platform : Platform,
  privateKey: PrivateKey,
  identity: Identity,
  addPublicKeys?: IdentityPublicKey[],
  disablePublicKeys?: number[],
  publicKeysDisabledAt?: number,
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

  await identityUpdateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

  const result = await dpp.stateTransition.validateBasic(identityUpdateTransition);

  if (!result.isValid()) {
    throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
  }

  return identityUpdateTransition;
}
