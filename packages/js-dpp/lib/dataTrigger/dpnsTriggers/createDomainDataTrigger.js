const hash = require('../../util/hash');

const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

const MAX_PRINTABLE_DOMAIN_NAME_LENGTH = 253;

/**
 * Data trigger for domain creation process
 *
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 * @param {Buffer} topLevelIdentity
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
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Full domain name length can not be more than 253 characters long',
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

  if (records.dashUniqueIdentityId
    && !context.getOwnerId().equals(records.dashUniqueIdentityId)
  ) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'ownerId doesn\'t match dashUniqueIdentityId',
      ),
    );
  }

  if (records.dashAliasIdentityId
    && !context.getOwnerId().equals(records.dashAliasIdentityId)) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'ownerId doesn\'t match dashAliasIdentityId',
      ),
    );
  }

  if (normalizedParentDomainName.length === 0
    && !context.getOwnerId().equals(topLevelIdentity)) {
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
      result.addError(
        new DataTriggerConditionError(
          documentTransition,
          context.getDataContract(),
          context.getOwnerId(),
          'Parent domain is not present',
        ),
      );

      return result;
    }

    if (subdomainRules.allowSubdomains === true) {
      result.addError(
        new DataTriggerConditionError(
          documentTransition,
          context.getDataContract(),
          context.getOwnerId(),
          'Allowing subdomains registration is forbidden for non top level domains',
        ),
      );
    }

    if (parentDomain.getData().subdomainRules.allowSubdomains === false
      && !context.getOwnerId().equals(parentDomain.getOwnerId())) {
      result.addError(
        new DataTriggerConditionError(
          documentTransition,
          context.getDataContract(),
          context.getOwnerId(),
          'The subdomain can be created only by the parent domain owner',
        ),
      );
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
