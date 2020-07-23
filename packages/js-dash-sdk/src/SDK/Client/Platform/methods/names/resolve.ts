import { hash } from '@dashevo/dpp/lib/util/multihashDoubleSHA256';

import { Platform } from "../../Platform";

/**
 * This method will allow you to resolve a DPNS record from its humanized name.
 * @param {string} name - the exact alphanumeric (2-63) value used for human-identification
 * @returns {Document} document
 */
export async function resolve(this: Platform, name: string): Promise<any> {
    const normalizedAndHashedName = hash(
        Buffer.from(name.toLowerCase()),
    ).toString('hex');

    const [document] = await this.documents.get('dpns.domain', {
        where: [
            ['nameHash', '==', normalizedAndHashedName],
        ],
    });

    return document === undefined ? null : document;
}

export default resolve;
