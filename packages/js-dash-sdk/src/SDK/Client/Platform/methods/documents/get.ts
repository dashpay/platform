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
    if (this.client.getApps().has(appName)) {
        const appDefinition = this.client.getApps().get(appName);

        if (!appDefinition.contract) {
            await this.contracts.get(appDefinition.contractId);
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

    if (!this.client.getApps().has(appName)) {
        throw new Error(`No app named ${appName} specified.`)
    }

    const appDefinition = this.client.getApps().get(appName);

    if (!appDefinition.contractId) {
        throw new Error(`Missing contract ID for ${appName}`)
    }

    // If not present, will fetch contract based on appName and contractId store in this.apps.
    await ensureAppContractFetched.call(this, appName);

    // @ts-ignore
    const rawDocuments = await this.client.getDAPIClient().platform.getDocuments(
        appDefinition.contractId,
        fieldType,
        opts
    );

    return Promise.all(
        rawDocuments.map((rawDocument) => this.dpp.document.createFromBuffer(rawDocument)),
    );
}

export default get;
