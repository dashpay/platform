const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

let Document;
let DataContract;
let ProtocolVersionValidator;
let DocumentValidator;
let DocumentFactory;

describe('DocumentBatchTransition', () => {
  let stateTransitionFixture;
  let dataContractFixture;
  let documentsFixture;
  let documentFactory;

  let mediumSecurityDocumentFixture;
  let masterSecurityDocumentFixture;
  let noSecurityLevelSpecifiedDocumentFixture;

  beforeEach(async function beforeEach() {
    ({
      Document,
      DataContract,
      ProtocolVersionValidator,
      DocumentFactory,
      DocumentValidator,
    } = await loadWasmDpp());
    const dataContractFixtureJs = getDataContractFixture();

    dataContractFixtureJs.documents.niceDocument
      .signatureSecurityLevelRequirement = IdentityPublicKey.SECURITY_LEVELS.MEDIUM;
    dataContractFixtureJs.documents.prettyDocument
      .signatureSecurityLevelRequirement = IdentityPublicKey.SECURITY_LEVELS.MASTER;

    dataContractFixture = new DataContract(dataContractFixtureJs.toObject());

    // 0 is niceDocument,
    // 1 and 2 are pretty documents,
    // 3 and 4 are indexed documents that do not have security level specified
    documentsFixture = getDocumentsFixture(dataContractFixtureJs).map((doc) => {
      const document = new Document(doc.toObject(), dataContractFixture.clone());
      document.setEntropy(doc.entropy);
      return document;
    });

    [
      mediumSecurityDocumentFixture, ,
      masterSecurityDocumentFixture, ,
      noSecurityLevelSpecifiedDocumentFixture,
    ] = documentsFixture;

    const protocolValidator = new ProtocolVersionValidator();
    const documentValidator = new DocumentValidator(protocolValidator);
    const stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    documentFactory = new DocumentFactory(1, documentValidator, stateRepositoryMock);

    stateTransitionFixture = documentFactory.createStateTransition({
      create: documentsFixture,
      replace: [],
      delete: [],
    });
  });

  describe('#getRequiredKeySecurityLevel', () => {
    it('should return the highest security level of all transitions - Rust', () => {
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

    it('should return default security level if no document has a security level defined - Rust', () => {
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
