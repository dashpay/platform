const DashPlatformProtocol = require('@dashevo/dpp');
const generateRandomId = require('@dashevo/dpp/lib/test/utils/generateRandomId');

const dpnsDocumentsSchema = require('../../src/schema/dpns-documents.json');

describe('DPNS Contract', () => {
  let dpp;
  let contract;
  let identityId;

  beforeEach(function beforeEach() {
    const fetchContractStub = this.sinon.stub();

    dpp = new DashPlatformProtocol({
      dataProvider: {
        fetchDataContract: fetchContractStub,
      },
    });

    identityId = generateRandomId();

    contract = dpp.dataContract.create(identityId, dpnsDocumentsSchema);

    fetchContractStub.resolves(contract);
  });

  it('should have a valid contract definition', async () => {
    const validationResult = await dpp.dataContract.validate(contract);

    expect(validationResult.isValid()).to.be.true();
  });

  describe('documents', () => {
    describe('preorder', () => {
      let preorderData;

      beforeEach(() => {
        preorderData = {
          saltedDomainHash: Buffer.alloc(32).toString('hex'),
        };
      });

      it('should throw validation error if `saltedDomainHash` is not specified', async () => {
        delete preorderData.saltedDomainHash;

        const preorder = dpp.document.create(contract, identityId, 'preorder', preorderData);

        const result = await dpp.document.validate(preorder);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('saltedDomainHash');
      });

      it('should throw validation error if `saltedDomainHash` is empty', async () => {
        preorderData.saltedDomainHash = '';

        const preorder = dpp.document.create(contract, identityId, 'preorder', preorderData);

        const result = await dpp.document.validate(preorder);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minLength');
        expect(error.dataPath).to.equal('.saltedDomainHash');
      });

      it('should throw validation error if additional properties are present', async () => {
        preorderData.someOtherProperty = 42;

        const preorder = dpp.document.create(contract, identityId, 'preorder', preorderData);

        const result = await dpp.document.validate(preorder);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperty).to.equal('someOtherProperty');
      });

      it('should successfuly validate preorder document if it is valid', async () => {
        const preorder = dpp.document.create(contract, identityId, 'preorder', preorderData);

        const result = await dpp.document.validate(preorder);

        expect(result.isValid()).to.be.true();
      });
    });

    describe('domain', () => {
      let domainData;

      beforeEach(() => {
        domainData = {
          nameHash: Buffer.alloc(32).toString('hex'),
          label: 'Wallet',
          normalizedLabel: 'wallet',
          normalizedParentDomainName: 'dash',
          preorderSalt: Buffer.alloc(32, 2).toString('hex'),
          records: {
            dashIdentity: generateRandomId(),
          },
        };
      });

      it('should throw validation error if `nameHash` is not specified', async () => {
        delete domainData.nameHash;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('nameHash');
      });

      it('should throw validation error if `nameHash` is empty', async () => {
        domainData.nameHash = '';

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minLength');
        expect(error.dataPath).to.equal('.nameHash');
      });

      it('should throw validation error if `label` is not specified', async () => {
        delete domainData.label;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('label');
      });

      it('should throw validation error if `label` is invalid', async () => {
        domainData.label = 'invalid label';

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('pattern');
        expect(error.dataPath).to.equal('.label');
      });

      it('should throw validation error if `normalizedLabel` is not specified', async () => {
        delete domainData.normalizedLabel;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('normalizedLabel');
      });

      it('should throw validation error if `normalizedLabel` is invalid', async () => {
        domainData.normalizedLabel = 'InValiD label';

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('pattern');
        expect(error.dataPath).to.equal('.normalizedLabel');
      });

      it('should throw validation error if `normalizedParentDomainName` is not specified', async () => {
        delete domainData.normalizedParentDomainName;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('normalizedParentDomainName');
      });

      it('should throw validation error if `preorderSalt` is not specified', async () => {
        delete domainData.preorderSalt;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('preorderSalt');
      });

      it('should throw validation error if `preorderSalt` is empty', async () => {
        domainData.preorderSalt = '';

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minLength');
        expect(error.dataPath).to.equal('.preorderSalt');
      });

      it('should throw validation error if `records` are not specified', async () => {
        delete domainData.records;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('records');
      });

      it('should throw validation error if `records` is empty', async () => {
        domainData.records = {};

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minProperties');
        expect(error.dataPath).to.equal('.records');
      });

      it('should throw validation error if `records` have a short `dashIdentity`', async () => {
        domainData.records = {
          dashIdentity: 'short indentity',
        };

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('minLength');
        expect(error.dataPath).to.equal('.records.dashIdentity');
      });

      it('should throw validation error if `records` have a long `dashIdentity`', async () => {
        domainData.records = {
          dashIdentity: Buffer.alloc(64).toString('hex'),
        };

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('maxLength');
        expect(error.dataPath).to.equal('.records.dashIdentity');
      });

      it('should throw validation error if additional properties are present', async () => {
        domainData.someOtherProperty = 42;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperty).to.equal('someOtherProperty');
      });

      it('should throw validation error if additional properties are present in records', async () => {
        domainData.records.someOtherProperty = 42;

        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.false();
        expect(result.errors).to.have.a.lengthOf(1);

        const [error] = result.errors;

        expect(error.name).to.equal('JsonSchemaError');
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.dataPath).to.equal('.records');
        expect(error.params.additionalProperty).to.equal('someOtherProperty');
      });

      it('shoud successfuly validate a domain document is it is valid', async () => {
        const domain = dpp.document.create(contract, identityId, 'domain', domainData);

        const result = await dpp.document.validate(domain);

        expect(result.isValid()).to.be.true();
      });
    });
  });
});
