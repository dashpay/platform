import { Identifier, Identity } from '@dashevo/wasm-dpp';
import broadcastStateTransition from '../../broadcastStateTransition';
import { Platform } from '../../Platform';
import { signStateTransition } from '../../signStateTransition';

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
