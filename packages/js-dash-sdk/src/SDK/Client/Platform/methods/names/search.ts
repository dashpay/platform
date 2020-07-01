import {Platform} from "../../Platform";

export async function search(this: Platform, labelPrefix: string, parentDomainName: string = '') {
    const normalizedParentDomainName = parentDomainName.toLowerCase();

    const documents = await this.documents.get('dpns.domain', {
        where: [
            ['normalizedParentDomainName', '==', normalizedParentDomainName],
            ['normalizedLabel', 'startsWith', labelPrefix],
        ],
    });

    return documents;
}

export default search;
