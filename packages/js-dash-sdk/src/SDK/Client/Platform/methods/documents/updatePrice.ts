import { Identity, ExtendedDocument } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';
/**
 * Transfer document in the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ExtendedDocument} document - document from the DAPI
 * @param {Identifier} receiver - identifier of the document recipient ownership
 * @param {Identifier} sender - identifier of the document owner
 */
export async function updatePrice(
  this: Platform,
  document: ExtendedDocument,
  amount: number,
  identity: Identity,
): Promise<any> {
  this.logger.debug(`[Document#transfer] Update price for document ${document.getId().toString()} to ${amount}`);
  await this.initialize();

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(identity.getId(), document.getDataContractId());

  const documentsBatchTransition = document
    .createUpdatePriceStateTransition(amount, BigInt(identityContractNonce));

  await signStateTransition(this, documentsBatchTransition, identity, 1);

  await broadcastStateTransition(this, documentsBatchTransition);
}

export default updatePrice;
