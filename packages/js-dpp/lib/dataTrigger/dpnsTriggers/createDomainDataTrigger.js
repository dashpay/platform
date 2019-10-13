const multihash = require('../../util/multihashDoubleSHA256');

const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH = 253;

/**
 * Data trigger for domain creation process
 *
 * @param {Document} document
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createDomainDataTrigger(document, context) {
  const {
    nameHash,
    label,
    normalizedLabel,
    normalizedParentDomainName,
    preorderSalt,
    records,
  } = document.getData();

  const result = new DataTriggerExecutionResult();

  const fullDomainName = `${normalizedLabel}.${normalizedParentDomainName}`;

  if (fullDomainName.length > MAX_PRINTABLE_DOMAIN_NAME_LENGTH) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'Full domain name length can not be more than 253 characters long',
      ),
    );
  }

  const isHashValidMultihash = multihash.validate(Buffer.from(nameHash, 'hex'));

  if (!isHashValidMultihash) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'nameHash is not a valid multihash',
      ),
    );
  }

  if (nameHash !== multihash.hash(Buffer.from(fullDomainName)).toString('hex')) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'Document nameHash doesn\'t match actual hash',
      ),
    );
  }

  if (normalizedLabel !== label.toLowerCase()) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'Normalized label doesn\'t match label',
      ),
    );
  }

  if (context.getUserId() !== records.dashIdentity) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'userId doesn\'t match dashIdentity',
      ),
    );
  }

  if (normalizedParentDomainName !== normalizedParentDomainName.toLowerCase()) {
    result.addError(
      new DataTriggerConditionError(
        document,
        context,
        'Parent domain name is not normalized (e.g. contains non-lowercase letter)',
      ),
    );
  }

  const parentDomainHash = multihash.hash(Buffer.from(normalizedParentDomainName))
    .toString('hex');

  const [parentDomain] = await context.getDataProvider().fetchDocuments(
    context.getDataContract().getId(),
    document.getType(),
    { where: [['nameHash', '==', parentDomainHash]] },
  );

  if (!parentDomain) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'Can\'t find parent domain matching parent hash',
      ),
    );
  }

  const saltedDomainHash = multihash.hash(Buffer.from(preorderSalt + nameHash, 'hex'))
    .toString('hex');

  const [preorderDocument] = await context.getDataProvider()
    .fetchDocuments(
      context.getDataContract().getId(),
      'preorder',
      { where: [['saltedDomainHash', '==', saltedDomainHash]] },
    );

  if (!preorderDocument) {
    result.addError(
      new DataTriggerConditionError(
        document, context, 'preorderDocument was not found',
      ),
    );
  }

  return result;
}

module.exports = createDomainDataTrigger;
