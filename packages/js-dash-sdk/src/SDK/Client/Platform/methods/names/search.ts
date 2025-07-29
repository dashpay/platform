import { Platform } from '../../Platform';

import convertToHomographSafeChars from '../../../../../utils/convertToHomographSafeChars';

/**
 *
 * @param {string} labelPrefix - label prefix to search for
 * @param {string} parentDomainName - parent domain name on which to perform the search
 * @returns Documents[] - The array of documents that match the search parameters.
 */
export async function search(this: Platform, labelPrefix: string, parentDomainName: string = '') {
  await this.initialize();

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    
    this.logger.debug(`[Names#search] Calling wasm-sdk getNameBySearch for "${labelPrefix}"`);
    
    // Call wasm-sdk getNameBySearch
    const result = await this.wasmSdk.getNameBySearch(
      labelPrefix,
      parentDomainName || undefined
    );
    
    if (!result) {
      return [];
    }
    
    // Convert the result to array of documents
    const documents = Array.isArray(result) ? result : [result];
    
    this.logger.debug(`[Names#search] Found ${documents.length} names via wasm-sdk`);
    
    return documents.map(doc => adapter.convertResponse(doc, 'document'));
  }

  const normalizedParentDomainName = convertToHomographSafeChars(parentDomainName);
  const normalizedLabelPrefix = convertToHomographSafeChars(labelPrefix);

  const documents = await this.documents.get('dpns.domain', {
    where: [
      ['normalizedParentDomainName', '==', normalizedParentDomainName],
      ['normalizedLabel', 'startsWith', normalizedLabelPrefix],
    ],
    orderBy: [
      ['normalizedLabel', 'asc'],
    ],
  });

  return documents;
}

export default search;
