const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const DocumentFactory = require('../../../../../lib/document/DocumentFactory');
const createDPPMock = require('../../../../../lib/test/mocks/createDPPMock');

describe('DocumentBatchTransition', () => {
  let stateTransitionFixture;
  let dataContractFixture;
  let documentsFixture;
  let documentFactory;
  let mediumSecurityDocumentFixture;
  let masterSecurityDocumentFixture;
  let noSecurityLevelSpecifiedDocumentFixture;

  beforeEach(() => {
    dataContractFixture = getDataContractFixture();

    dataContractFixture.documents.niceDocument
      .keySecurityLevelRequirement = IdentityPublicKey.SECURITY_LEVELS.MEDIUM;
    dataContractFixture.documents.prettyDocument
      .keySecurityLevelRequirement = IdentityPublicKey.SECURITY_LEVELS.MASTER;

    // 0 is niceDocument,
    // 1 and 2 are pretty documents,
    // 3 and 4 are indexed documents that do not have security level specified
    documentsFixture = getDocumentsFixture(dataContractFixture);
    [
      mediumSecurityDocumentFixture,,
      masterSecurityDocumentFixture,,
      noSecurityLevelSpecifiedDocumentFixture,
    ] = documentsFixture;

    documentFactory = new DocumentFactory(
      createDPPMock(),
      () => {},
      () => {},
    );

    stateTransitionFixture = documentFactory.createStateTransition({
      create: documentsFixture,
      replace: [],
      delete: [],
    });
  });

  describe('#getRequiredKeySecurityLevel', () => {
    it('should return the highest security level of all transitions', () => {
      stateTransitionFixture = documentFactory.createStateTransition({
        create: [mediumSecurityDocumentFixture],
        replace: [],
        delete: [],
      });

      // Nice document has medium security level
      expect(stateTransitionFixture.getKeySecurityLevelRequirement())
        .to.be.equal(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

      stateTransitionFixture = documentFactory.createStateTransition({
        create: [mediumSecurityDocumentFixture, masterSecurityDocumentFixture],
        replace: [],
        delete: [],
      });

      // Should be the highest security level out of MEDIUM and MASTER
      expect(stateTransitionFixture.getKeySecurityLevelRequirement())
        .to.be.equal(IdentityPublicKey.SECURITY_LEVELS.MASTER);
    });

    it('should return default security level if no document has a security level defined', () => {
      stateTransitionFixture = documentFactory.createStateTransition({
        create: [noSecurityLevelSpecifiedDocumentFixture],
        replace: [],
        delete: [],
      });

      // Should be the default level, which is HIGH
      expect(stateTransitionFixture.getKeySecurityLevelRequirement())
        .to.be.equal(IdentityPublicKey.SECURITY_LEVELS.HIGH);
    });
  });
});
