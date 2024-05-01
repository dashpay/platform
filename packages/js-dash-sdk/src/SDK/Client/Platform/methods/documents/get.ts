import { Identifier, ExtendedDocument, Metadata } from '@dashevo/wasm-dpp';
import { GetDocumentsResponse } from '@dashevo/dapi-client/lib/methods/platform/getDocuments/GetDocumentsResponse';
import NotFoundError from '@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError';
import { Platform } from '../../Platform';
import { QueryOptions, WhereCondition } from '../../types';

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

  if (isSystemIdentifier || isPropertyIdentifier) {
    if (Array.isArray(propertyValue)) {
      convertedPropertyValue = propertyValue.map((id) => Identifier.from(id));
    } else if (typeof propertyValue === 'string') {
      convertedPropertyValue = Identifier.from(propertyValue);
    }
  }

  return [propertyName, operator, convertedPropertyValue];
}

/**
 * Get documents from the platform
 *
 * @param {Platform} this bound instance class
 * @param {string} typeLocator type locator
 * @param {QueryOptions} opts - MongoDB style query
 * @returns documents
 */
export async function get(this: Platform, typeLocator: string, opts: QueryOptions): Promise<any> {
  this.logger.debug(`[Documents#get] Get document(s) for "${typeLocator}"`);
  if (!typeLocator.includes('.')) throw new Error('Accessing to field is done using format: appName.fieldName');

  await this.initialize();

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
      .map((whereCondition) => convertIdentifierProperties(whereCondition, binaryProperties));
  }

  if (opts.startAt instanceof ExtendedDocument) {
    opts.startAt = opts.startAt.getId();
  } else if (typeof opts.startAt === 'string') {
    opts.startAt = Identifier.from(opts.startAt);
  }

  if (opts.startAfter instanceof ExtendedDocument) {
    opts.startAfter = opts.startAfter.getId();
  } else if (typeof opts.startAfter === 'string') {
    opts.startAfter = Identifier.from(opts.startAfter);
  }

  let documentsResponse: GetDocumentsResponse;

  try {
    documentsResponse = await this.client.getDAPIClient().platform.getDocuments(
      appDefinition.contractId,
      fieldType,
      opts,
    );
  } catch (e) {
    // TODO: NotFoundError is returing only in case if contract or document type is not found?
    //  If contract or document type is invalid we should throw an error
    if (e instanceof NotFoundError) {
      this.logger.debug(`[Documents#get] Obtained 0 documents for "${typeLocator}"`);
      return [];
    }

    throw e;
  }

  const rawDocuments = documentsResponse.getDocuments();

  const result = await Promise.all(
    rawDocuments.map(async (rawDocument) => {
      const document = await this.dpp.document
        .createExtendedDocumentFromDocumentBuffer(
          rawDocument as Uint8Array,
          fieldType,
          appDefinition.contract,
        );

      let metadata;
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
