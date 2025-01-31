import { ExtendedDocument, Identifier } from '@dashevo/wasm-dpp';
import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

class DocumentTransitionParams {
  receiver?: Identifier;

  price?: bigint;
}

/**
 * Broadcast document onto the platform
 *
 * @param {Object} documents
 * @param {ExtendedDocument[]} [documents.create]
 * @param {ExtendedDocument[]} [documents.replace]
 * @param {ExtendedDocument[]} [documents.delete]
 * @param {Identity} identity
 * @param options {DocumentTransitionParams} optional params for NFT functions
 */
export default async function broadcast(
  this: Platform,
  documents: {
    create?: ExtendedDocument[],
    replace?: ExtendedDocument[],
    delete?: ExtendedDocument[],
    transfer?: ExtendedDocument[],
    updatePrice?: ExtendedDocument[],
    purchase?: ExtendedDocument[],
  },
  identity: any,
  options?: DocumentTransitionParams,
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
  ][0]?.getDataContractId();

  if (!dataContractId) {
    throw new Error('Data contract ID is not found');
  }

  if (documents.transfer?.length && !options?.receiver) {
    throw new Error('Receiver identity is not found for transfer transition');
  }

  if (documents.updatePrice?.length && !options?.price) {
    throw new Error('Price must be provided for UpdatePrice operation');
  }

  if (documents.purchase?.length) {
    if (!options?.price && !options?.receiver) {
      throw new Error('Price and Receiver must be provided for Purchase operation');
    }

    documents.purchase.forEach((document) => document.setOwnerId(options.receiver));
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
    options?.receiver,
    options?.price,
  );

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
    transfer: documents.transfer?.length || 0,
  });

  return documentsBatchTransition;
}
