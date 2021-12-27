const DashPlatformProtocol = require('@dashevo/dpp');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const rewardSharingContractSchema = require('../../schema/reward-share-documents.json');

describe('Reward sharing contract', () => {
  let dpp;
  let contract;
  let identityId;
  let rewardShare;

  beforeEach(async function beforeEach() {
    const rewardSharingContractStub = this.sinon.stub();

    dpp = new DashPlatformProtocol({
      stateRepository: {
        rewardSharingDataContract: rewardSharingContractStub,
      },
    });

    await dpp.initialize();

    identityId = generateRandomIdentifier();

    contract = dpp.dataContract.create(identityId, rewardSharingContractSchema);

    rewardSharingContractStub.resolves(contract);

    rewardShare = {
      payToId: generateRandomIdentifier(),
      percentage: 500,
    };
  });

  it('should have a valid contract definition', async function shouldHaveValidContract() {
    this.timeout(5000);

    const validationResult = await dpp.dataContract.validate(contract);

    expect(validationResult.isValid()).to.be.true();
  });

  describe('payToId', () => {
    it('should be defined', () => {
      delete rewardShare.payToId;

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.errors).to.have.a.lengthOf(1);

        const [error] = e.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('payToId');
      }
    });

    it('should have no less than 32 bytes', () => {
      rewardShare.payToId = Buffer.alloc(31);

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.getErrors()).to.have.a.lengthOf(1);

        const [error] = e.getErrors();

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minItems');
        expect(error.instancePath).to.equal('/payToId');
      }
    });

    it('should have no more than 32 bytes', async () => {
      rewardShare.payToId = Buffer.alloc(33);

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.getErrors()).to.have.a.lengthOf(1);

        const [error] = e.getErrors();

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('maxItems');
        expect(error.instancePath).to.equal('/payToId');
      }
    });
  });

  describe('percentage', () => {
    it('should be defined', () => {
      delete rewardShare.percentage;

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.errors).to.have.a.lengthOf(1);

        const [error] = e.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('percentage');
      }
    });

    it('should not be less than 1', () => {
      rewardShare.percentage = 0;

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.errors).to.have.a.lengthOf(1);
        const [error] = e.errors;
        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minimum');
        expect(error.instancePath).to.equal('/percentage');
      }
    });

    it('should not be more than 10000', () => {
      rewardShare.percentage = 10001;

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.errors).to.have.a.lengthOf(1);
        const [error] = e.errors;
        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('maximum');
        expect(error.instancePath).to.equal('/percentage');
      }
    });

    it('should be a number', () => {
      rewardShare.percentage = '10';

      try {
        dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

        expect.fail('should throw error');
      } catch (e) {
        expect(e.name).to.equal('InvalidDocumentError');
        expect(e.errors).to.have.a.lengthOf(1);
        const [error] = e.errors;
        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('type');
        expect(error.instancePath).to.equal('/percentage');
        expect(error.params.type).to.equal('integer');
      }
    });
  });

  it('should should not have additional properties', () => {
    rewardShare.someOtherProperty = 42;

    try {
      dpp.document.create(contract, identityId, 'rewardShare', rewardShare);

      expect.fail('should throw error');
    } catch (e) {
      expect(e.name).to.equal('InvalidDocumentError');
      expect(e.errors).to.have.a.lengthOf(1);

      const [error] = e.errors;

      expect(error.name).to.equal('JsonSchemaError');
      expect(error.keyword).to.equal('additionalProperties');
      expect(error.params.additionalProperty).to.equal('someOtherProperty');
    }
  });
});
