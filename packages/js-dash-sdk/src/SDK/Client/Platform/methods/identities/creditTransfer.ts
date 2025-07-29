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

  recipientId = Identifier.from(recipientId);

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    // Get the identity's private key for signing
    const account = await this.client.getWalletAccount();
    
    // Get the transfer key (index 3)
    const { privateKey: transferPrivateKey } = account.identities
      .getIdentityHDKeyById(identity.getId().toString(), 3);
    
    // Convert private key to WIF format
    const privateKeyWIF = adapter.convertPrivateKeyToWIF(transferPrivateKey);
    
    // Convert identity to hex format
    const identityHex = identity.toBuffer().toString('hex');
    
    // Call wasm-sdk identityCreditTransfer
    const result = await this.wasmSdk.identityCreditTransfer(
      identityHex,
      privateKeyWIF,
      recipientId.toString(),
      amount
    );
    
    this.logger.debug(`[Identity#creditTransfer] Transferred ${amount} credits from ${identity.getId().toString()} to ${recipientId.toString()}`);
    
    return result.success !== false;
  }

  // Legacy implementation - will be removed once migration is complete
  const { dpp } = this;

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