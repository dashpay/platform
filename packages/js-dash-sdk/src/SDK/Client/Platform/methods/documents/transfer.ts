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
export async function transfer(
  this: Platform,
  document: ExtendedDocument,
  receiver: Identity,
  sender: Identity,
): Promise<any> {
  this.logger.debug(`[Document#transfer] Transfer document ${document.getId().toString()} from ${sender.getId().toString} to {${receiver.getId().toString()}`);
  await this.initialize();

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(sender.getId(), document.getDataContractId());

  const documentsBatchTransition = document
    .createTransferStateTransition(receiver.getId(), BigInt(identityContractNonce));

  await signStateTransition(this, documentsBatchTransition, sender, 1);

  await broadcastStateTransition(this, documentsBatchTransition);
}

export default transfer;
