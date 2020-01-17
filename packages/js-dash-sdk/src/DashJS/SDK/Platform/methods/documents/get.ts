import {Platform} from "../../Platform";

declare interface fetchOpts {
    where: any;
    orderBy: any;
    limit: number;
    startAt: number;
    startAfter: number;
}

export async function get(this: Platform, typeLocator: string, opts: fetchOpts): Promise<any> {
    const appNames = Object.keys(this.apps);

    //We can either provide of type `dashpay.profile` or if only one schema provided, of type `profile`.
    const [appName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [appNames[0], typeLocator];
    // FIXME: we may later want a hashmap of schemas and contract IDs

    if (!this.apps[appName]) {
        throw new Error(`No app named ${appName} specified.`)
    }
    const app = this.apps[appName];
    if (!app.contractId) {
        throw new Error(`Missing contract ID for ${appName}`)
    }
    const contractId = app.contractId;
    try{
        // @ts-ignore
        const rawDataList = await this.client.getDocuments(contractId, fieldType, opts);
        const documents: any[] = [];

        for (const rawData of rawDataList) {
            try {
                const doc = await this.dpp.document.createFromSerialized(rawData, {skipValidation: true})
                documents.push(doc);
            } catch (e) {
                console.error('Document creation: failure', e);
            }
        }
        return documents
    } catch (e) {
        console.error(`Document creation: unable to get documents of ${contractId}`);
        throw e;
    }
}

export default get;
