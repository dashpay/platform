import Identity from "@dashevo/dpp/lib/identity/Identity";
import { Platform } from "../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"
import { signStateTransition } from '../../signStateTransition';

import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Update platform identities
 *
 * @param {Platform} this - bound instance class
 * @param {Identity} identity - identity to update
 * @param {{add: IdentityPublicKey[]; disable: IdentityPublicKey[]}} publicKeys - public keys to add
 * @param {Object<string, any>} privateKeys - public keys to add
 *
 * @returns {boolean}
 */
export async function update(
  this: Platform,
  identity: Identity,
  publicKeys: { add?: IdentityPublicKey[]; disable?: IdentityPublicKey[] },
  privateKeys: { string, any },
  ): Promise<any> {
  await this.initialize();

  const { dpp } = this;

  const identityUpdateTransition = dpp.identity.createIdentityUpdateTransition(
    identity,
    publicKeys,
  );

  const signerKeyIndex = 0;

  // Create key proofs
  if (identityUpdateTransition.getPublicKeysToAdd()) {
    const signerKey = identity.getPublicKeys()[signerKeyIndex];

    // must be run sequentially! will not work with Promise.all!
    // more info at https://jrsinclair.com/articles/2019/how-to-run-async-js-in-parallel-or-sequential/

    const starterPromise = Promise.resolve(null);

    await identityUpdateTransition.getPublicKeysToAdd().reduce(
      (previousPromise, publicKey) => previousPromise.then(async () => {
        const privateKey = privateKeys[publicKey.getId()];

        if (!privateKey) {
          throw new Error(`Private key for key ${publicKey.getId()} not found`);
        }

        identityUpdateTransition.setSignaturePublicKeyId(signerKey.getId());

        await identityUpdateTransition.signByPrivateKey(privateKey, publicKey.getType());

        publicKey.setSignature(identityUpdateTransition.getSignature());

        identityUpdateTransition.setSignature(undefined);
        identityUpdateTransition.setSignaturePublicKeyId(undefined);
      }),
      starterPromise,
    );
  }

  await signStateTransition(this, identityUpdateTransition, identity, signerKeyIndex);

  const result = await dpp.stateTransition.validateBasic(identityUpdateTransition);

  if (!result.isValid()) {
    throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
  }

  // Broadcast ST
  await broadcastStateTransition(this, identityUpdateTransition);

  return true;
}

export default update;
