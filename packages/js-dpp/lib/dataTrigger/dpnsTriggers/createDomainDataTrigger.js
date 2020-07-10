const bs58 = require('bs58');

const multihash = require('../../util/multihashDoubleSHA256');

const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH = 253;

/**
 * Data trigger for domain creation process
 *
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 * @param {string} topLevelIdentity
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createDomainDataTrigger(documentTransition, context, topLevelIdentity) {
  const {
    nameHash,
    label,
    normalizedLabel,
    normalizedParentDomainName,
    preorderSalt,
    records,
  } = documentTransition.getData();

  const result = new DataTriggerExecutionResult();

  let fullDomainName = normalizedLabel;
  if (normalizedParentDomainName.length > 0) {
    fullDomainName = `${normalizedLabel}.${normalizedParentDomainName}`;
  }

  if (fullDomainName.length > MAX_PRINTABLE_DOMAIN_NAME_LENGTH) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Full domain name length can not be more than 253 characters long',
      ),
    );
  }

  const isHashValidMultihash = multihash.validate(Buffer.from(nameHash, 'hex'));

  if (!isHashValidMultihash) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'nameHash is not a valid multihash',
      ),
    );
  }

  if (nameHash !== multihash.hash(Buffer.from(fullDomainName)).toString('hex')) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Document nameHash doesn\'t match actual hash',
      ),
    );
  }

  if (normalizedLabel !== label.toLowerCase()) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Normalized label doesn\'t match label',
      ),
    );
  }

  if (context.getOwnerId() !== records.dashIdentity) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'ownerId doesn\'t match dashIdentity',
      ),
    );
  }

  // TODO: Move this to DPNS contract
  if (normalizedParentDomainName !== normalizedParentDomainName.toLowerCase()) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Parent domain name is not normalized (e.g. contains non-lowercase letter)',
      ),
    );
  }

  if (normalizedParentDomainName.length === 0 && context.getOwnerId() !== topLevelIdentity) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Can\'t create top level domain for this identity',
      ),
    );
  }

  if (normalizedParentDomainName.length > 0) {
    const parentDomainHash = multihash.hash(Buffer.from(normalizedParentDomainName))
      .toString('hex');

    const [parentDomain] = await context.getStateRepository().fetchDocuments(
      context.getDataContract().getId(),
      documentTransition.getType(),
      { where: [['nameHash', '==', parentDomainHash]] },
    );

    if (!parentDomain) {
      result.addError(
        new DataTriggerConditionError(
          documentTransition,
          context.getDataContract(),
          context.getOwnerId(),
          'Can\'t find parent domain matching parent hash',
        ),
      );
    }
  }

  const saltedPreorderDomainNameHashBuffer = Buffer.concat([
    bs58.decode(preorderSalt),
    Buffer.from(nameHash, 'hex'),
  ]);

  const saltedDomainHash = multihash.hash(saltedPreorderDomainNameHashBuffer)
    .toString('hex');

  const [preorderDocument] = await context.getStateRepository()
    .fetchDocuments(
      context.getDataContract().getId(),
      'preorder',
      { where: [['saltedDomainHash', '==', saltedDomainHash]] },
    );

  if (!preorderDocument) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'preorderDocument was not found',
      ),
    );
  }

  return result;
}

module.exports = createDomainDataTrigger;
