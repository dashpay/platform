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
    const normalizedParentDomainName = '';
    const normalizedLabel = name.toLowerCase();

    const domainDocument = await dpp.document.create(
      dataContract,
      ownerId,
      'domain',
      {
        label: name,
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

    await documentRepository.store(domainDocument, true);
  }

  return registerTopLevelDomain;
}

module.exports = registerTopLevelDomainFactory;
