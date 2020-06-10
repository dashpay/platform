import {Platform} from "../../Platform";

/**
 * @param {any} [where] - where
 * @param {any} [orderBy] - order by
 * @param {number} [limit] - limit
 * @param {number} [startAt] - start value (included)
 * @param {number} [startAfter] - start value (not included)
 */
declare interface fetchOpts {
    where: any;
    orderBy: any;
    limit: number;
    startAt: number;
    startAfter: number;
}

/**
 * Prefetch contract
 *
 * @param {Platform} this bound instance class
 * @param {string} appName of the contract to fetch
 */
const ensureAppContractFetched = async function (this: Platform, appName) {
    if (this.apps[appName]) {
        if (!this.apps[appName].contract) {
            const app = this.apps[appName];
            // contracts.get deals with settings contract into this.apps[appName]
            await this.contracts.get(app.contractId);
        }
    }
}

/**
 * Get documents from the platform
 *
 * @param {Platform} this bound instance class
 * @param {string} typeLocator type locator
 * @param {fetchOpts} opts - MongoDB style query
 * @returns documents
 */
export async function get(this: Platform, typeLocator: string, opts: fetchOpts): Promise<any> {
    if (!typeLocator.includes('.')) throw new Error('Accessing to field is done using format: appName.fieldName');

    // locator is of `dashpay.profile` with dashpay the app and profile the field.
    const [appName, fieldType] = typeLocator.split('.');
    // FIXME: we may later want a hashmap of schemas and contract IDs

    if (!this.apps[appName]) {
        throw new Error(`No app named ${appName} specified.`)
    }
    const app = this.apps[appName];
    if (!app.contractId) {
        throw new Error(`Missing contract ID for ${appName}`)
    }

    const contractId = app.contractId;
    // If not present, will fetch contract based on appName and contractId store in this.apps.
    await ensureAppContractFetched.call(this, appName);
    // @ts-ignore
    const rawDataList = await this.client.getDAPIClient().getDocuments(contractId, fieldType, opts);
    const documents: any[] = [];

    for (const rawData of rawDataList) {
        const doc = await this.dpp.document.createFromSerialized(rawData, {skipValidation: true});
        documents.push(doc);
    }
    return documents
}

export default get;
