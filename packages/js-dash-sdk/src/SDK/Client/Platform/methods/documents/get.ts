import Identifier from '@dashevo/dpp/lib/Identifier';
import Metadata from "@dashevo/dpp/lib/Metadata";

import {Platform} from "../../Platform";

/**
 * @param {WhereCondition[]} [where] - where
 * @param {OrderByCondition[]} [orderBy] - order by
 * @param {number} [limit] - limit
 * @param {number} [startAt] - start value (included)
 * @param {number} [startAfter] - start value (not included)
 */
declare interface fetchOpts {
    where?: WhereCondition[];
    orderBy?: OrderByCondition[];
    limit?: number;
    startAt?: number;
    startAfter?: number;
}

type OrderByCondition = [
    string,
    'asc' | 'desc',
];

type WhereCondition = [
    string,
    '<' | '<=' | '==' | '>' | '>=' | 'in' | 'startsWith' | 'elementMatch' | 'length' | 'contains',
    WhereCondition|any,
]

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
 * Convert where condition identifier properties
 *
 * @param {WhereCondition} whereCondition
 * @param {Object} binaryProperties
 * @param {null|string} [parentProperty=null]
 *
 * @return {WhereCondition}
 */
function convertIdentifierProperties(whereCondition: WhereCondition, binaryProperties: Record<string, any>, parentProperty: null|string = null) {
    const [propertyName, operator, propertyValue] = whereCondition;

    const fullPropertyName = parentProperty ? `${parentProperty}.${propertyName}`: propertyName;

    if (operator === 'elementMatch') {
        return [
            propertyName,
            operator,
            convertIdentifierProperties(
                propertyValue,
                binaryProperties,
                fullPropertyName,
            ),
        ];
    }

    let convertedPropertyValue = propertyValue;

    const property = binaryProperties[fullPropertyName];

    const isPropertyIdentifier = property && property.contentMediaType === Identifier.MEDIA_TYPE;
    const isSystemIdentifier = ['$id', '$ownerId'].includes(propertyName);

    if (isSystemIdentifier || (isPropertyIdentifier && typeof propertyValue === 'string')) {
        convertedPropertyValue = Identifier.from(propertyValue);
    }

    return [propertyName, operator, convertedPropertyValue];
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

    await this.initialize();

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

    if (opts.where) {
        const binaryProperties = appDefinition.contract.getBinaryProperties(fieldType);

        opts.where = opts.where.map((whereCondition) => convertIdentifierProperties(whereCondition, binaryProperties));
    }

    // @ts-ignore
    const documentsResponse = await this.client.getDAPIClient().platform.getDocuments(
        appDefinition.contractId,
        fieldType,
        opts
    );

    const rawDocuments = documentsResponse.getDocuments();

    return Promise.all(
        rawDocuments.map(async (rawDocument) => {
            const document = await this.dpp.document.createFromBuffer(rawDocument);

            let metadata = null;
            const responseMetadata = documentsResponse.getMetadata();
            if (responseMetadata) {
                metadata = new Metadata({
                    blockHeight: responseMetadata.getHeight(),
                    coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
                });
            }
            document.setMetadata(metadata);

            return document;
        }),
    );
}

export default get;
