const hash = require('../../util/hash');

const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH = 253;

/**
 * Data trigger for domain creation process
 *
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 * @param {Identifier|Buffer} topLevelIdentity
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createDomainDataTrigger(documentTransition, context, topLevelIdentity) {
  const {
    label,
    normalizedLabel,
    normalizedParentDomainName,
    preorderSalt,
    records,
    subdomainRules,
  } = documentTransition.getData();

  const result = new DataTriggerExecutionResult();

  let fullDomainName = normalizedLabel;
  if (normalizedParentDomainName.length > 0) {
    fullDomainName = `${normalizedLabel}.${normalizedParentDomainName}`;
  }

  if (fullDomainName.length > MAX_PRINTABLE_DOMAIN_NAME_LENGTH) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Full domain name length can not be more than ${MAX_PRINTABLE_DOMAIN_NAME_LENGTH} characters long but got ${fullDomainName.length}`,
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  if (normalizedLabel !== label.toLowerCase()) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      'Normalized label doesn\'t match label',
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  if (records.dashUniqueIdentityId
    && !context.getOwnerId().equals(records.dashUniqueIdentityId)
  ) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `ownerId ${context.getOwnerId()} doesn't match dashUniqueIdentityId ${records.dashUniqueIdentityId}`,
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  if (records.dashAliasIdentityId
    && !context.getOwnerId().equals(records.dashAliasIdentityId)) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `ownerId ${context.getOwnerId()} doesn't match dashAliasIdentityId ${records.dashAliasIdentityId}`,
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  if (normalizedParentDomainName.length === 0
    && !context.getOwnerId().equals(topLevelIdentity)) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      'Can\'t create top level domain for this identity',
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  if (normalizedParentDomainName.length > 0) {
    const parentDomainSegments = normalizedParentDomainName.split('.');

    const parentDomainLabel = parentDomainSegments[0];
    const grandParentDomainName = parentDomainSegments.slice(1).join('.');

    const [parentDomain] = await context.getStateRepository().fetchDocuments(
      context.getDataContract().getId(),
      documentTransition.getType(),
      {
        where: [
          ['normalizedParentDomainName', '==', grandParentDomainName],
          ['normalizedLabel', '==', parentDomainLabel],
        ],
      },
    );

    if (!parentDomain) {
      const error = new DataTriggerConditionError(
        context.getDataContract().getId().toBuffer(),
        documentTransition.getId().toBuffer(),
        'Parent domain is not present',
      );

      error.setOwnerId(context.getOwnerId());
      error.setDocumentTransition(documentTransition);

      result.addError(error);

      return result;
    }

    if (subdomainRules.allowSubdomains === true) {
      const error = new DataTriggerConditionError(
        context.getDataContract().getId().toBuffer(),
        documentTransition.getId().toBuffer(),
        'Allowing subdomains registration is forbidden for non top level domains',
      );

      error.setOwnerId(context.getOwnerId());
      error.setDocumentTransition(documentTransition);

      result.addError(error);
    }

    if (parentDomain.getData().subdomainRules.allowSubdomains === false
      && !context.getOwnerId().equals(parentDomain.getOwnerId())) {
      const error = new DataTriggerConditionError(
        context.getDataContract().getId().toBuffer(),
        documentTransition.getId().toBuffer(),
        'The subdomain can be created only by the parent domain owner',
      );

      error.setOwnerId(context.getOwnerId());
      error.setDocumentTransition(documentTransition);

      result.addError(error);
    }
  }

  const saltedDomainBuffer = Buffer.concat([
    preorderSalt,
    Buffer.from(fullDomainName),
  ]);

  const saltedDomainHash = hash(saltedDomainBuffer);

  const [preorderDocument] = await context.getStateRepository()
    .fetchDocuments(
      context.getDataContract().getId(),
      'preorder',
      { where: [['saltedDomainHash', '==', saltedDomainHash]] },
    );

  if (!preorderDocument) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      'preorderDocument was not found',
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  return result;
}

module.exports = createDomainDataTrigger;
