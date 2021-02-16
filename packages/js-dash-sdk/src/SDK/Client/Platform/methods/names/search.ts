import {Platform} from "../../Platform";

/**
 *
 * @param {string} labelPrefix - label prefix to search for
 * @param {string} parentDomainName - parent domain name on which to perform the search
 * @returns Documents[] - The array of documents that match the search parameters.
 */
export async function search(this: Platform, labelPrefix: string, parentDomainName: string = '') {
    const normalizedParentDomainName = parentDomainName.toLowerCase();
    const normalizedLabelPrefix = labelPrefix.toLowerCase();

    const documents = await this.documents.get('dpns.domain', {
        where: [
            ['normalizedParentDomainName', '==', normalizedParentDomainName],
            ['normalizedLabel', 'startsWith', normalizedLabelPrefix],
        ],
    });

    return documents;
}

export default search;
