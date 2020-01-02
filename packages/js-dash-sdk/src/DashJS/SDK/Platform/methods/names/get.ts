import {Platform} from "../../Platform";

export async function get(this: Platform, id: string): Promise<any> {
    const queryOpts = {
        where: [
            ['normalizedLabel', '==', id.toLowerCase()],
            ['normalizedParentDomainName', '==', 'dash'],
        ],
    };
    try{
        const documents = await this.documents.get('dpns.domain', queryOpts);
        return (documents[0] !== undefined) ? documents[0] : null;
    }catch (e) {
        throw e;
    }
}

export default get;
