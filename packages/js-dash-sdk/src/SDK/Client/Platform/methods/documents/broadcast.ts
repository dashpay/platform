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
      [dataContractId.toString()]: identityContractNonce.toString(),
    },
  });

  this.logger.silly('[Document#broadcast] Created documents batch transition');

  // From protocol version 14 the id of a new document commits to the identity
  // contract nonce of its create transition, so it only exists now that the
  // transition is built: the id a document was created with is a placeholder.
  // Hand the final ids back so callers can keep using `document.getId()`.
  if (documents.create) {
    // Only create transitions carry an entropy. Documents of one type that share an
    // entropy still get ids of their own (their nonces differ), so keep a queue per
    // type and entropy: the factory keeps the creates in the order they were given.
    const createdIds = new Map<string, any[]>();
    documentsBatchTransition.getTransitions().forEach((transition) => {
      const entropy = transition.getEntropy && transition.getEntropy();
      if (!entropy) {
        return;
      }

      const key = `${transition.getType()}/${Buffer.from(entropy).toString('hex')}`;
      createdIds.set(key, [...(createdIds.get(key) || []), transition.getId()]);
    });

    documents.create.forEach((document) => {
      const key = `${document.getType()}/${Buffer.from(document.getEntropy()).toString('hex')}`;
      const id = (createdIds.get(key) || []).shift();
      if (id) {
        document.setId(id);
      }
    });
  }

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
