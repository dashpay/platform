const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DocumentRepository} documentRepository
 * @param {DocumentRepository} previousDocumentRepository
 * @param {RootTree} rootTree
 * @param {RootTree} previousRootTree
 * @param {Identifier} dashPreorderDocumentId
 * @param {Identifier} dashDomainDocumentId
 * @param {Buffer} dashPreorderSalt
 *
 * @return {registerTopLevelDomain}
 */
function registerTopLevelDomainFactory(
  dpp,
  documentRepository,
  previousDocumentRepository,
  rootTree,
  previousRootTree,
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
   * @param {Date} genesisTime
   *
   * @return {Promise<void>}
   */
  async function registerTopLevelDomain(name, dataContract, ownerId, genesisTime) {
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
    preorderDocument.createdAt = genesisTime;

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
    domainDocument.createdAt = genesisTime;

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
