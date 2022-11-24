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
   * @param {BlockInfo} blockInfo
   * @param {GroveDBTransaction} transaction
   *
   * @return {Promise<void>}
   */
  async function registerTopLevelDomain(name, dataContract, ownerId, blockInfo, transaction) {
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
    domainDocument.createdAt = new Date(blockInfo.timeMs);

    await documentRepository.create(domainDocument, blockInfo, {
      transaction,
    });
  }

  return registerTopLevelDomain;
}

module.exports = registerTopLevelDomainFactory;
