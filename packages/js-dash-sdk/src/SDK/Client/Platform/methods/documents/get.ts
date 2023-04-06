import { Platform } from '../../Platform';

// TODO(wasm-dpp): provide type definitions from wasm-dpp
let Identifier;
let Document;
let Metadata;

/**
 * @param {WhereCondition[]} [where] - where
 * @param {OrderByCondition[]} [orderBy] - order by
 * @param {number} [limit] - limit
 * @param {string|Buffer|Document|Identifier} [startAt] - start value (included)
 * @param {string|Buffer|Document|Identifier} [startAfter] - start value (not included)
 */
declare interface FetchOpts {
  where?: WhereCondition[];
  orderBy?: OrderByCondition[];
  limit?: number;
  startAt?: string | Buffer | typeof Document | typeof Identifier;
  startAfter?: string | Buffer | typeof Document | typeof Identifier;
}

type OrderByCondition = [
  string,
  'asc' | 'desc',
];

type WhereCondition = [
  string,
  '<' | '<=' | '==' | '>' | '>=' | 'in' | 'startsWith' | 'elementMatch' | 'length' | 'contains',
  WhereCondition | any,
];

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
};

/**
 * Convert where condition identifier properties
 *
 * @param {WhereCondition} whereCondition
 * @param {Object} binaryProperties
 * @param {null|string} [parentProperty=null]
 *
 * @return {WhereCondition}
 */
function convertIdentifierProperties(
  whereCondition: WhereCondition,
  binaryProperties: Record<string, any>,
  parentProperty: null | string = null,
) {
  const [propertyName, operator, propertyValue] = whereCondition;

  const fullPropertyName = parentProperty ? `${parentProperty}.${propertyName}` : propertyName;

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
 * @param {FetchOpts} opts - MongoDB style query
 * @returns documents
 */
export async function get(this: Platform, typeLocator: string, opts: FetchOpts): Promise<any> {
  this.logger.debug(`[Documents#get] Get document(s) for "${typeLocator}"`);
  if (!typeLocator.includes('.')) throw new Error('Accessing to field is done using format: appName.fieldName');

  await this.initialize();

  // TODO(wasm-dpp): remove when dppModule is typed
  // @ts-ignore
  ({ Identifier, Document, Metadata } = this.dppModule);

  // locator is of `dashpay.profile` with dashpay the app and profile the field.
  const [appName, fieldType] = typeLocator.split('.');
  // FIXME: we may later want a hashmap of schemas and contract IDs

  if (!this.client.getApps().has(appName)) {
    throw new Error(`No app named ${appName} specified.`);
  }

  const appDefinition = this.client.getApps().get(appName);

  if (!appDefinition.contractId) {
    throw new Error(`Missing contract ID for ${appName}`);
  }

  // If not present, will fetch contract based on appName and contractId store in this.apps.
  await ensureAppContractFetched.call(this, appName);
  this.logger.silly(`[Documents#get] Ensured app contract is fetched "${typeLocator}"`);

  if (opts.where) {
    const binaryProperties = appDefinition.contract.getBinaryProperties(fieldType);

    opts.where = opts.where
      .map((whereCondition) => convertIdentifierProperties(
        whereCondition, binaryProperties,
      ));
  }

  if (opts.startAt instanceof Document) {
    opts.startAt = opts.startAt.getId();
  } else if (typeof opts.startAt === 'string') {
    opts.startAt = Identifier.from(opts.startAt);
  }

  if (opts.startAfter instanceof Document) {
    opts.startAfter = opts.startAfter.getId();
  } else if (typeof opts.startAfter === 'string') {
    opts.startAfter = Identifier.from(opts.startAfter);
  }

  // @ts-ignore
  const documentsResponse = await this.client.getDAPIClient().platform.getDocuments(
    appDefinition.contractId,
    fieldType,
    opts,
  );

  const rawDocuments = documentsResponse.getDocuments();

  this.logger.silly(`[Documents#get] Obtained ${rawDocuments.length} raw document(s)"`);

  const result = await Promise.all(
    rawDocuments.map(async (rawDocument) => {
      const document = await this.wasmDpp.document.createFromBuffer(rawDocument);

      let metadata = null;
      const responseMetadata = documentsResponse.getMetadata();
      if (responseMetadata) {
        metadata = new Metadata({
          blockHeight: responseMetadata.getHeight(),
          coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
          timeMs: responseMetadata.getTimeMs(),
          protocolVersion: responseMetadata.getProtocolVersion(),
        });
      }
      document.setMetadata(metadata);

      return document;
    }),
  );

  this.logger.debug(`[Documents#get] Obtained ${result.length} document(s) for "${typeLocator}"`);

  return result;
}

export default get;
