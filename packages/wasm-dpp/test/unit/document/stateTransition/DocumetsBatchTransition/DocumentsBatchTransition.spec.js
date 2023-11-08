// TODO: tests can't be done using comparison with JS version anymore
//   consider restoring them using the new wasm version

// const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
// const getDataContractFixture =
// require('../../../../../lib/test/fixtures/getDataContractFixture');
// const {
//   default: loadWasmDpp,
//   StateTransitionTypes,
//   getLatestProtocolVersion,
// } = require('../../../../../dist');
//
// let DocumentFactory;
// let ExtendedDocument;
// let DocumentValidator;
// let ProtocolVersionValidator;

describe.skip('DocumentsBatchTransition', () => {
  // let stateTransitionJs;
  // let stateTransition;
  // let documentsJs;
  // let documents;
  // let dataContractJs;
  // let dataContract;
  // let factoryJs;
  //
  // beforeEach(async () => {
  //   ({
  //     ProtocolVersionValidator, DocumentValidator, DocumentFactory,
  //     ExtendedDocument,
  //   } = await loadWasmDpp());
  // });
  //
  // beforeEach(async () => {
  //   dataContractJs = getDataContractFixture();
  //   // dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());
  //
  //   documentsJs = getDocumentsFixture(dataContractJs);
  //   documents = documentsJs.map((d) => {
  //     const doc = new ExtendedDocument(d.toObject(), dataContract);
  //     doc.setEntropy(d.entropy);
  //     return doc;
  //   });
  //
  //   const protocolVersionValidatorRs = new ProtocolVersionValidator();
  //   const documentValidatorRs = new DocumentValidator(protocolVersionValidatorRs);
  //   const factory = new DocumentFactory(1, documentValidatorRs, {});
  //   factoryJs = new DocumentFactoryJs(createDPPMock(), undefined, undefined);
  //
  //   stateTransitionJs = factoryJs.createStateTransition({
  //     create: documentsJs,
  //   });
  //
  //   stateTransition = factory.createStateTransition({
  //     create: documents,
  //   });
  // });
  //
  // describe('#getProtocolVersion', () => {
  //   it('should return the current protocol version', () => {
  //     const result = stateTransition.getProtocolVersion();
  //
  //     expect(result).to.equal(protocolVersion.latestVersion);
  //   });
  // });
  //
  // describe('#getType', () => {
  //   it('should return State Transition type', () => {
  //     const result = stateTransition.getType();
  //
  //     expect(result).to.equal(stateTransitionTypes.DOCUMENTS_BATCH);
  //   });
  // });
  //
  // describe('#getTransitions', () => {
  //   it('should return document transitions', () => {
  //     const result = stateTransition.getTransitions().map((t) => t.toObject());
  //
  //     expect(result).to.deep.equal(stateTransitionJs.getTransitions().map((t) => t.toObject()));
  //   });
  // });
  //
  // describe('#toJSON', () => {
  //   it('should return State Transition as JSON', () => {
  //     expect(stateTransition.toJSON()).to.deep.equal({
  //       protocolVersion: protocolVersion.latestVersion,
  //       type: stateTransitionTypes.DOCUMENTS_BATCH,
  //       ownerId: documentsJs[0].getOwnerId().toString(),
  //       transitions: stateTransitionJs.getTransitions().map((d) => d.toJSON()),
  //       signaturePublicKeyId: undefined,
  //       signature: undefined,
  //     });
  //   });
  // });
  //
  // describe('#toObject', () => {
  //   it('should return State Transition as plain object', () => {
  //     expect(stateTransition.toObject()).to.deep.equal({
  //       protocolVersion: protocolVersion.latestVersion,
  //       type: stateTransitionTypes.DOCUMENTS_BATCH,
  //       ownerId: documentsJs[0].getOwnerId(),
  //       transitions: stateTransitionJs.getTransitions().map((d) => d.toObject()),
  //       signaturePublicKeyId: undefined,
  //       signature: undefined,
  //     });
  //   });
  // });
  //
  // describe('#toBuffer', () => {
  //   it('should return the same bytes as JS version', () => {
  //     const buffer = stateTransition.toBuffer();
  //     expect(buffer).to.be.instanceOf(Buffer);
  //     expect(buffer).to.have.length(23960);
  //   });
  // });
  //
  // describe('#hash', () => {
  //   it('should return the same hash as the JS version', () => {
  //     const hashJs = stateTransitionJs.hash();
  //     const hash = stateTransitionJs.hash();
  //
  //     expect(hash).to.deep.equal(hashJs);
  //   });
  // });
  //
  // describe('#getOwnerId', () => {
  //   it('should return owner id', async () => {
  //     const result = stateTransition.getOwnerId();
  //
  //     expect(result.toBuffer()).to.deep.equal(getDocumentsFixture.ownerId.toBuffer());
  //   });
  // });
  //
  // describe('#getModifiedDataIds', () => {
  //   it('should return ids of affected documents', () => {
  //     const expectedIds = documentsJs.map((doc) => doc.getId().toBuffer());
  //     const result = stateTransition.getModifiedDataIds().map((id) => id.toBuffer());
  //
  //     expect(result.length).to.be.equal(10);
  //     expect(result).to.be.deep.equal(expectedIds);
  //   });
  // });
  //
  // describe('#isDataContractStateTransition', () => {
  //   it('should return false', () => {
  //     expect(stateTransition.isDataContractStateTransition()).to.be.false();
  //   });
  // });
  //
  // describe('#isDocumentStateTransition', () => {
  //   it('should return true', () => {
  //     expect(stateTransition.isDocumentStateTransition()).to.be.true();
  //   });
  // });
  //
  // describe('#isIdentityStateTransition', () => {
  //   it('should return false', () => {
  //     expect(stateTransition.isIdentityStateTransition()).to.be.false();
  //   });
  // });
});
