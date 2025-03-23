import { ExtendedDocument, Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

interface DocumentTransitionParams {
  receiver?: Identifier;
  price?: bigint;
}

interface DocumentSubmittable {
  document: ExtendedDocument;
  params?: DocumentTransitionParams;
}

/**
 * Broadcast document onto the platform
 *
 * @param {Object} documents
 * @param {DocumentSubmittable[]} [documents.create]
 * @param {DocumentSubmittable[]} [documents.replace]
 * @param {DocumentSubmittable[]} [documents.delete]
 * @param {DocumentSubmittable[]} [documents.transfer]
 * @param {DocumentSubmittable[]} [documents.updatePrice]
 * @param {DocumentSubmittable[]} [documents.purchase]
 * @param {Identity} identity
 */
export default async function broadcast(
  this: Platform,
  documents: {
    create?: DocumentSubmittable[],
    replace?: DocumentSubmittable[],
    delete?: DocumentSubmittable[],
    transfer?: DocumentSubmittable[],
    updatePrice?: DocumentSubmittable[],
    purchase?: DocumentSubmittable[],
  },
  identity: any,
): Promise<any> {
  this.logger.debug('[Document#broadcast] Broadcast documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
    transfer: documents.transfer?.length || 0,
    updatePrice: documents.updatePrice?.length || 0,
    purchase: documents.purchase?.length || 0,
  });
  await this.initialize();

  const { dpp } = this;

  const identityId = identity.getId();
  const dataContractId = [
    ...(documents.create || []),
    ...(documents.replace || []),
    ...(documents.delete || []),
    ...(documents.transfer || []),
    ...(documents.updatePrice || []),
    ...(documents.purchase || []),
  ][0]?.document.getDataContractId();

  if (!dataContractId) {
    throw new Error('Data contract ID is not found');
  }

  if (documents.transfer?.length && documents.transfer
    .some(({ params }) => !params?.receiver)) {
    throw new Error('Receiver Identity is not found for Transfer transition');
  }

  if (documents.updatePrice?.length && documents.updatePrice
    .some(({ params }) => !params?.price)) {
    throw new Error('Price must be provided for UpdatePrice operation');
  }

  if (documents.purchase?.length) {
    if (documents.purchase
      .some(({ params }) => !params?.price || !params?.receiver)) {
      throw new Error('Receiver and Price must be provided for Purchase operation');
    } else {
      documents.purchase.forEach(({ document, params }) => document.setOwnerId(params!.receiver));
    }
  }

  const identityContractNonce = await this.nonceManager
    .bumpIdentityContractNonce(identityId, dataContractId);

  const identityNonceObj = {
    [identityId.toString()]: {
      [dataContractId.toString()]: identityContractNonce.toString(),
    },
  };

  const documentsBatchTransition = dpp.document.createStateTransition(
    documents,
    identityNonceObj,
  );

  this.logger.silly('[Document#broadcast] Created documents batch transition');

  await signStateTransition(this, documentsBatchTransition, identity, 1);

  // Broadcast state transition also wait for the result to be obtained
  await broadcastStateTransition(this, documentsBatchTransition);

  // Acknowledge documents identifiers to handle retry attempts to mitigate
  // state transition propagation lag
  if (documents.create) {
    documents.create.forEach(({ document }) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.acknowledgeKey(documentLocator);
    });
  }

  // Forget documents identifiers to not retry on them anymore
  if (documents.delete) {
    documents.delete.forEach(({ document }) => {
      const documentLocator = `${document.getDataContractId().toString()}/${document.getType()}`;
      this.fetcher.forgetKey(documentLocator);
    });
  }

  this.logger.debug('[Document#broadcast] Broadcasted documents', {
    create: documents.create?.length || 0,
    replace: documents.replace?.length || 0,
    delete: documents.delete?.length || 0,
    transfer: documents.transfer?.length || 0,
  });

  return documentsBatchTransition;
}
