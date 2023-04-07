import { Platform } from '../../Platform';
/**
 *
 * @param {string} labelPrefix - label prefix to search for
 * @param {string} parentDomainName - parent domain name on which to perform the search
 * @returns Documents[] - The array of documents that match the search parameters.
 */
export declare function search(this: Platform, labelPrefix: string, parentDomainName?: string): Promise<any>;
export default search;
