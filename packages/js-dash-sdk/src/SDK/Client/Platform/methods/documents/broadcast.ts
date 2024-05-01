import { ExtendedDocument } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

/**
 * Broadcast document onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Object} documents
 * @param {ExtendedDocument[]} [documents.create]
 * @param {ExtendedDocument[]} [documents.replace]
 * @param {ExtendedDocument[]} [documents.delete]
 * @param identity - identity
 */
export default async function broadcast(
  this: Platform,
  documents: {
    create?: ExtendedDocument[],
    replace?: ExtendedDocument[],
    delete?: ExtendedDocument[]
  },
  identity: any,
): Promise<any> {
  this.logger.debug('[Document#broadcast] Broadcast documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
  });
  await this.initialize();

  const { dpp } = this;

  const identityId = identity.getId();
  const dataContractId = [
    ...(documents.create || []),
    ...(documents.replace || []),
    ...(documents.delete || []),
  ][0]?.getDataContractId();

  if (!dataContractId) {
    throw new Error('Data contract ID is not found');
  }

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(identityId, dataContractId);

  const documentsBatchTransition = dpp.document.createStateTransition(documents, {
    [identityId.toString()]: {
      [dataContractId.toString()]: identityContractNonce,
    },
  });

  this.logger.silly('[Document#broadcast] Created documents batch transition');

  await signStateTransition(this, documentsBatchTransition, identity, 1);

  // Broadcast state transition also wait for the result to be obtained
  await broadcastStateTransition(this, documentsBatchTransition);

  // Acknowledge documents identifiers to handle retry attempts to mitigate
  // state transition propagation lag
  if (documents.create) {
    documents.create.forEach((document) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.acknowledgeKey(documentLocator);
    });
  }

  // Forget documents identifiers to not retry on them anymore
  if (documents.delete) {
    documents.delete.forEach((document) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.forgetKey(documentLocator);
    });
  }

  this.logger.debug('[Document#broadcast] Broadcasted documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
  });

  return documentsBatchTransition;
}
