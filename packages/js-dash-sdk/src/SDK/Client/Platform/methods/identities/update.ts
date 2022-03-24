// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import { Platform } from "../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"
import { signStateTransition } from '../../signStateTransition';

import broadcastStateTransition from "../../broadcastStateTransition";

/**
 * Update platform identities
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {{create: IdentityPublicKey[]; delete: IdentityPublicKey[]}} publicKeys - public keys to add
 *
 * @returns {boolean}
 */
export async function update(
  this: Platform,
  identityId: Identifier | string,
  publicKeys: { create?: IdentityPublicKey[]; delete?: IdentityPublicKey[] },
  ): Promise<any> {
  await this.initialize();

  const { dpp } = this;

  identityId = Identifier.from(identityId);

  const identity = await this.identities.get(identityId);

  if (identity === null) {
    throw new Error(`Identity with ID ${identityId.toString()} not found`)
  }

  const identityUpdateTransition = dpp.identity.createIdentityUpdateTransition(
    identity,
    publicKeys,
  );

  await signStateTransition(this, identityUpdateTransition, identity);

  const result = await dpp.stateTransition.validateBasic(identityUpdateTransition);

  if (!result.isValid()) {
    throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
  }

  // Broadcast ST
  await broadcastStateTransition(this, identityUpdateTransition);

  return true;
}

export default update;
