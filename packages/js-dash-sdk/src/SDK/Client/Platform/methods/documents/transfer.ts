import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';

/**
 * Transfer document in the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} typeLocator - type locator
 * @param identity - identity
 * @param {Object} [data] - options
 */
export async function transfer(
  this: Platform,
  documentId: string,
  identity: any,
): Promise<any> {
  this.logger.debug(`[Document#transfer] Transfer document`);
  await this.initialize();

  const { dpp } = this;

  const document = await this.documents.get(documentId);

  this.logger.silly(`[Document#create] Obtained document ${document.getId()}`);

  if (document === null) {
    throw new Error(`Document ${documentId} not found. Ensure contractId ${documentId} is correct.`);
  }

  document.setOwnerId(identity);

  const transition = dpp.document.createStateTransition(
    { transfer: [document] },
    identity.getId(),
  );

  await broadcastStateTransition(this, transition);
}

export default transfer;
