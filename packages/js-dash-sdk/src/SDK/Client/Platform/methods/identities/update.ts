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
 *
 * @returns {boolean}
 */
export async function update(
  this: Platform,
  identity: Identity,
  publicKeys: { add?: IdentityPublicKey[]; disable?: IdentityPublicKey[] },
  ): Promise<any> {
  await this.initialize();

  const { dpp } = this;

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
