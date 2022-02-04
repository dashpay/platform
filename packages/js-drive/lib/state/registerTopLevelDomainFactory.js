/**
 * @param {DashPlatformProtocol} dpp
 * @param {DocumentRepository} documentRepository
 * @param {Identifier} dashDomainDocumentId
 * @param {Buffer} dashPreorderSalt
 *
 * @return {registerTopLevelDomain}
 */
function registerTopLevelDomainFactory(
  dpp,
  documentRepository,
  // dashPreorderDocumentId,
  dashDomainDocumentId,
  dashPreorderSalt,
) {
  /**
   * @typedef registerTopLevelDomain
   *
   * @param {string} name
   * @param {DataContract} dataContract
   * @param {Identifier} ownerId
   * @param {Date} genesisDate
   *
   * @return {Promise<void>}
   */
  async function registerTopLevelDomain(name, dataContract, ownerId, genesisDate) {
    const nameLabels = name.split('.');

    const normalizedParentDomainName = nameLabels
      .slice(1)
      .join('.')
      .toLowerCase();

    const [label] = nameLabels;
    const normalizedLabel = label.toLowerCase();

    // const saltedDomainHash = hash(
    //   Buffer.concat([
    //     dashPreorderSalt,
    //     Buffer.from(normalizedLabel),
    //   ]),
    // );

    // const preorderDocument = await dpp.document.create(
    //   dataContract,
    //   ownerId,
    //   'preorder',
    //   {
    //     saltedDomainHash,
    //   },
    // );
    //
    // preorderDocument.id = dashPreorderDocumentId;
    // preorderDocument.createdAt = genesisDate;

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
          allowSubdomains: true,
        },
      },
    );

    domainDocument.id = dashDomainDocumentId;
    domainDocument.createdAt = genesisDate;

    // await documentRepository.store(preorderDocument, true);
    await documentRepository.store(domainDocument, true);
  }

  return registerTopLevelDomain;
}

module.exports = registerTopLevelDomainFactory;
