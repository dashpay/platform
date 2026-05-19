import wasmDpp from '@dashevo/wasm-dpp';
const { Identifier, Identity } = wasmDpp;
import type {
  Identifier as IdentifierType,
  Identity as IdentityType,
} from '@dashevo/wasm-dpp';
type Identifier = IdentifierType;
type Identity = IdentityType;
import broadcastStateTransition from '../../broadcastStateTransition.js';
import { Platform } from '../../Platform.js';
import { signStateTransition } from '../../signStateTransition.js';

export async function creditTransfer(
  this: Platform,
  identity: Identity,
  recipientId: Identifier | string,
  amount: number,
): Promise<any> {
  this.logger.debug(`[Identity#creditTransfer] credit transfer from ${identity.getId().toString()} to ${recipientId.toString()} with amount ${amount}`);
  await this.initialize();

  const { dpp } = this;

  recipientId = Identifier.from(recipientId);

  const identityNonce = await this.nonceManager.bumpIdentityNonce(identity.getId());

  const identityCreditTransferTransition = dpp.identity
    .createIdentityCreditTransferTransition(
      identity,
      recipientId,
      BigInt(amount),
      BigInt(identityNonce),
    );

  this.logger.silly('[Identity#creditTransfer] Created IdentityCreditTransferTransition');

  const signerKeyIndex = 3;

  await signStateTransition(this, identityCreditTransferTransition, identity, signerKeyIndex);

  // Skipping validation because it's already done above
  await broadcastStateTransition(this, identityCreditTransferTransition, {
    skipValidation: true,
  });

  this.logger.silly('[Identity#creditTransfer] Broadcasted IdentityCreditTransferTransition');

  return true;
}

export default creditTransfer;
