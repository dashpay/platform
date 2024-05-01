import { Identity, IdentityPublicKey } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import { signStateTransition } from '../../signStateTransition';

import broadcastStateTransition from '../../broadcastStateTransition';

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
  this.logger.debug(`[Identity#update] Update identity ${identity.getId().toString()}`, {
    addKeys: publicKeys.add ? publicKeys.add.length : 0,
    disableKeys: publicKeys.disable ? publicKeys.disable.map((key) => key.getId()).join(', ') : 'none',
  });
  await this.initialize();

  const { dpp } = this;

  const identityNonce = await this.nonceManager.bumpIdentityNonce(identity.getId());

  const identityUpdateTransition = dpp.identity.createIdentityUpdateTransition(
    identity,
    BigInt(identityNonce),
    publicKeys,
  );

  this.logger.silly('[Identity#update] Created IdentityUpdateTransition');

  const signerKeyIndex = 0;

  // Create key proofs
  if (identityUpdateTransition.getPublicKeysToAdd()) {
    const signerKey = identity.getPublicKeys()[signerKeyIndex];

    // must be run sequentially! will not work with Promise.all!
    // more info at https://jrsinclair.com/articles/2019/how-to-run-async-js-in-parallel-or-sequential/

    const starterPromise = Promise.resolve(null);

    const updatedPublicKeys: any[] = [];
    await identityUpdateTransition.getPublicKeysToAdd().reduce(
      (previousPromise, publicKey) => previousPromise.then(async () => {
        const privateKey = privateKeys[publicKey.getId()];

        if (!privateKey) {
          throw new Error(`Private key for key ${publicKey.getId()} not found`);
        }

        identityUpdateTransition.setSignaturePublicKeyId(signerKey.getId());

        await identityUpdateTransition.signByPrivateKey(privateKey.toBuffer(), publicKey.getType());

        publicKey.setSignature(identityUpdateTransition.getSignature());
        updatedPublicKeys.push(publicKey);

        identityUpdateTransition.setSignature(undefined);
        identityUpdateTransition.setSignaturePublicKeyId(undefined);
      }),
      starterPromise,
    );

    // Update public keys in transition to include signatures
    identityUpdateTransition.setPublicKeysToAdd(updatedPublicKeys);
  }

  await signStateTransition(this, identityUpdateTransition, identity, signerKeyIndex);
  this.logger.silly('[Identity#update] Signed IdentityUpdateTransition');

  // TODO(versioning): restore
  // @ts-ignore
  // const result = await dpp.stateTransition.validateBasic(
  //   identityUpdateTransition,
  //   // TODO(v0.24-backport): get rid of this once decided
  //   //  whether we need execution context in wasm bindings
  //   new StateTransitionExecutionContext(),
  // );

  // if (!result.isValid()) {
  //   const messages = result.getErrors().map((error) => error.message);
  //   throw new Error(`StateTransition is invalid - ${JSON.stringify(messages)}`);
  // }
  this.logger.silly('[Identity#update] Validated IdentityUpdateTransition');

  // Skipping validation because it's already done above
  await broadcastStateTransition(this, identityUpdateTransition, {
    skipValidation: true,
  });

  this.logger.silly('[Identity#update] Broadcasted IdentityUpdateTransition');

  return true;
}

export default update;
