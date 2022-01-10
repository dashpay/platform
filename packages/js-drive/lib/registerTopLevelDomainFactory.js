const crypto = require('crypto');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const { asValue } = require('awilix');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DocumentIndexedStoreRepository} documentRepository
 * @param {DocumentIndexedStoreRepository} previousDocumentRepository
 * @param {RootTree} rootTree
 * @param {RootTree} previousRootTree
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {cloneToPreviousStoreTransactions} cloneToPreviousStoreTransactions
 * @param {AwilixContainer} container
 *
 * @return {registerTopLevelDomain}
 */
function registerTopLevelDomainFactory(
  dpp,
  documentRepository,
  previousDocumentRepository,
  rootTree,
  previousRootTree,
  blockExecutionStoreTransactions,
  cloneToPreviousStoreTransactions,
  container,
  documentEntropy,
  documentCreatedAt,
  dashPreorderDocumentId,
  dashDomainDocumentId,
  dashPreorderSalt,
) {
  /**
   * @typedef registerTopLevelDomain
   *
   * @param {string} name
   * @param {DataContract} dataContract
   * @param {Identifier} ownerId
   *
   * @return {Promise<void>}
   */
  async function registerTopLevelDomain(name, dataContract, ownerId) {
    await blockExecutionStoreTransactions.start();

    const previousBlockExecutionStoreTransactions = await cloneToPreviousStoreTransactions(
      blockExecutionStoreTransactions,
    );

    container.register({
      previousBlockExecutionStoreTransactions: asValue(previousBlockExecutionStoreTransactions),
    });

    await blockExecutionStoreTransactions.commit();

    const nameLabels = name.split('.');

    const normalizedParentDomainName = nameLabels
      .slice(1)
      .join('.')
      .toLowerCase();

    const [label] = nameLabels;
    const normalizedLabel = label.toLowerCase();

    const isSecondLevelDomain = normalizedParentDomainName.length > 0;

    const fullDomainName = isSecondLevelDomain
      ? `${normalizedLabel}.${normalizedParentDomainName}`
      : normalizedLabel;

    const saltedDomainHash = hash(
      Buffer.concat([
        dashPreorderSalt,
        Buffer.from(fullDomainName),
      ]),
    );

    const preorderDocument = await dpp.document.create(
      dataContract,
      ownerId,
      'preorder',
      {
        saltedDomainHash,
      },
    );

    preorderDocument.id = dashPreorderDocumentId;
    preorderDocument.entropy = documentEntropy;
    preorderDocument.createdAt = documentCreatedAt;

    const domainDocument = await dpp.document.create(
      dataContract,
      ownerId,
      'domain',
      {
        label,
        normalizedLabel,
        normalizedParentDomainName,
        preorderSalt: dashPreorderSalt,
        records: {
          dashAliasIdentityId: ownerId,
        },
        subdomainRules: {
          allowSubdomains: !isSecondLevelDomain,
        },
      },
    );

    domainDocument.id = dashDomainDocumentId;
    domainDocument.entropy = documentEntropy;
    domainDocument.createdAt = documentCreatedAt;

    await documentRepository.store(preorderDocument);
    await documentRepository.store(domainDocument);

    await previousDocumentRepository.store(preorderDocument);
    await previousDocumentRepository.store(domainDocument);

    rootTree.rebuild();
    previousRootTree.rebuild();
  }

  return registerTopLevelDomain;
}

module.exports = registerTopLevelDomainFactory;
