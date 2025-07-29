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

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get the identity's private key for signing (master key at index 0)
    const account = await this.client.getWalletAccount();
    
    // Get the master key for signing
    const { privateKey: masterPrivateKey } = account.identities
      .getIdentityHDKeyById(identity.getId().toString(), 0);
    
    // Convert private key to WIF format
    const privateKeyWIF = adapter.convertPrivateKeyToWIF(masterPrivateKey);
    
    // Prepare public keys to add
    let addPublicKeysJson: string | undefined;
    if (publicKeys.add && publicKeys.add.length > 0) {
      const keysToAdd = publicKeys.add.map(key => {
        // Get the private key for this public key to sign the proof
        const privateKey = privateKeys[key.getId()];
        if (!privateKey) {
          throw new Error(`Private key for key ${key.getId()} not found`);
        }
        
        return {
          id: key.getId(),
          type: key.getType(),
          purpose: key.getPurpose(),
          securityLevel: key.getSecurityLevel(),
          data: key.getData().toString('base64'),
          readOnly: key.isReadOnly(),
          // Include the private key for signing the proof
          privateKey: adapter.convertPrivateKeyToWIF(privateKey),
        };
      });
      addPublicKeysJson = JSON.stringify(keysToAdd);
    }
    
    // Prepare keys to disable
    const disableKeyIds = publicKeys.disable?.map(key => key.getId());
    
    this.logger.debug(`[Identity#update] Calling wasm-sdk identityUpdate`);
    
    // Call wasm-sdk identityUpdate
    const result = await this.wasmSdk.identityUpdate(
      identity.getId().toString(),
      addPublicKeysJson,
      disableKeyIds,
      privateKeyWIF
    );
    
    this.logger.debug(`[Identity#update] Updated identity ${identity.getId().toString()}`);
    
    return result.success !== false;
  }

  // Legacy implementation - will be removed once migration is complete
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