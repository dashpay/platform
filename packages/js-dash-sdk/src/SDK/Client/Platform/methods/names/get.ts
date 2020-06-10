import {Platform} from "../../Platform";

/**
 * Get names from the platform
 * @param {Platform} this - bound instance class
 * @param {string} id - id
 * @returns names
 */
export async function get(this: Platform, id: string): Promise<any> {
    const queryOpts = {
        where: [
            ['normalizedLabel', '==', id.toLowerCase()],
            ['normalizedParentDomainName', '==', 'dash'],
        ],
    };
    const documents = await this.documents.get('dpns.domain', queryOpts);
    return (documents[0] !== undefined) ? documents[0] : null;
};

export default get;
