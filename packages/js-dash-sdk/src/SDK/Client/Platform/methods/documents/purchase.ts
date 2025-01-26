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
export async function purchase(
  this: Platform,
  document: ExtendedDocument,
  amount: number,
  buyer: Identity,
  seller: Identity,
): Promise<any> {
  this.logger.debug(`[Document#transfer] Update price for document ${document.getId().toString()} to ${amount}`);
  await this.initialize();

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(buyer.getId(), document.getDataContractId());

  const documentsBatchTransition = document
    .createPurchaseStateTransition(buyer.getId(), amount, BigInt(identityContractNonce));

  await signStateTransition(this, documentsBatchTransition, buyer, 1);

  console.log(documentsBatchTransition.toBuffer().toString('hex'));

  await broadcastStateTransition(this, documentsBatchTransition);
}

export default purchase;
