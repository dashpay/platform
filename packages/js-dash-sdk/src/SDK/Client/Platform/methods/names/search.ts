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
