import { Platform } from '../../Platform';

const convertToBase58chars = require('@dashevo/dpp/lib/util/convertToBase58chars');

/**
 *
 * @param {string} labelPrefix - label prefix to search for
 * @param {string} parentDomainName - parent domain name on which to perform the search
 * @returns Documents[] - The array of documents that match the search parameters.
 */
export async function search(this: Platform, labelPrefix: string, parentDomainName: string = '') {
  await this.initialize();

  const normalizedParentDomainName = convertToBase58chars(parentDomainName.toLowerCase());
  const normalizedLabelPrefix = convertToBase58chars(labelPrefix.toLowerCase());

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
