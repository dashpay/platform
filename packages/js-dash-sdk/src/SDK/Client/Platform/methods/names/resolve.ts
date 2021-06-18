import { Platform } from "../../Platform";

/**
 * This method will allow you to resolve a DPNS record from its humanized name.
 * @param {string} name - the exact alphanumeric (2-63) value used for human-identification
 * @returns {Document} document
 */
export async function resolve(this: Platform, name: string): Promise<any> {
    await this.initialize();

    // setting up variables in case of TLD registration
    let normalizedLabel = name.toLowerCase();
    let normalizedParentDomainName = '';

    // in case of subdomain registration
    // we should split label and parent domain name
    if (name.includes('.')) {
        const segments = name.toLowerCase().split('.');

        normalizedLabel = segments[0];
        normalizedParentDomainName = segments.slice(1).join('.');
    }

    const [document] = await this.documents.get('dpns.domain', {
        where: [
            ['normalizedParentDomainName', '==', normalizedParentDomainName],
            ['normalizedLabel', '==', normalizedLabel],
        ],
    });

    return document === undefined ? null : document;
}

export default resolve;
