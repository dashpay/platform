const { PrivateKey } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

const DataContract = require('../../../lib/dataContract/DataContract');
const Identity = require('../../../lib/identity/Identity');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractCreateTransition;
  let documentsBatchTransition;
  let stateRepositoryMock;
  let dataContract;
  let identityPublicKey;

  beforeEach(function beforeEach() {
    const privateKeyModel = new PrivateKey();
    const privateKey = privateKeyModel.toBuffer();
    const publicKey = privateKeyModel.toPublicKey().toBuffer().toString('base64');
    const publicKeyId = 1;

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);

    dataContract = getDocumentsFixture.dataContract;

    dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });
    dataContractCreateTransition.sign(identityPublicKey, privateKey);

    const documentTransitions = getDocumentTransitionsFixture({
      create: getDocumentsFixture(),
    });

    documentsBatchTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });
    documentsBatchTransition.sign(identityPublicKey, privateKey);

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);
    const getBalance = this.sinonSandbox.stub().returns(10000);

    const identity = {
      getPublicKeyById,
      type: 2,
      getBalance,
    };

    const timeInSeconds = Math.ceil(new Date().getTime() / 1000);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);
    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
      time: {
        seconds: timeInSeconds,
      },
    });

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromObject(
          dataContractCreateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toJSON(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from plain object', async () => {
      const result = await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractCreateTransition.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromSerialized(
          dataContractCreateTransition.serialize(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromSerialized(
        dataContractCreateTransition.serialize(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from string', async () => {
      const result = await dpp.stateTransition.createFromSerialized(
        dataContractCreateTransition.serialize(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractCreateTransition.toJSON());
    });
  });

  describe('validate', async () => {
    it('should return invalid result if State Transition structure is invalid', async function it() {
      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const rawStateTransition = dataContractCreateTransition.toJSON();
      delete rawStateTransition.protocolVersion;

      const result = await dpp.stateTransition.validate(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateDataSpy).to.not.be.called();
    });

    it('should validate Data Contract ST structure and data', async function it() {
      const validateStructureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateStructure',
      );

      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(dataContractCreateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(dataContractCreateTransition);
    });

    it('should validate Documents ST structure and data', async function it() {
      stateRepositoryMock.fetchDocuments.resolves([]);

      stateRepositoryMock.fetchDataContract.resolves(dataContract);
      stateRepositoryMock.fetchIdentity.resolves({
        getPublicKeyById: this.sinonSandbox.stub().returns(identityPublicKey),
      });

      const validateStructureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateStructure',
      );

      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const result = await dpp.stateTransition.validate(
        documentsBatchTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(documentsBatchTransition);
      expect(validateDataSpy).to.be.calledOnceWith(documentsBatchTransition);
    });

    it('should not cache contract with the same schema it preventing further sts from being validated', async () => {
      const now = new Date().getTime();
      const st = {
        protocolVersion: 0,
        type: 1,
        signature: 'H5v2t8blUDwJ284sLV7ZBlmjEAuviK1EzhiqoiXLrI+fYPF2JeAv4IGH/doLTEaDp/PKXDvY5gC7gn9fjR7eVh0=',
        signaturePublicKeyId: 0,
        ownerId: 'Di94QuVennkE4FfTQHJF2MJhqehLbmodSn5Uzy5u4zHL',
        transitions: [
          {
            $action: 0,
            $dataContractId: '295xRRRMGYyAruG39XdAibaU9jMAzxhknkkAxFE7uVkW',
            $id: 'DhwqCBK82fmoxYaw8z5oTiKTPPWiCE95YJTj1zjHWpaG',
            $type: 'preorder',
            $entropy: 'yfLGvfKr3Y3ahtkeEKY3wTFz2zNjmsrwbj',
            saltedDomainHash: '562088f2e19881fe8c05da623463582fd84a644489e570a1ea3fcd716b28f11ed4f7',
            $createdAt: now,
            $updatedAt: now,
          },
        ],
      };

      const contract = new DataContract({
        $id: '295xRRRMGYyAruG39XdAibaU9jMAzxhknkkAxFE7uVkW',
        $schema: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
        ownerId: 'Czcr8PwPbXBCu1Jzu54MnC4urbdtrnKwsswUYeD2gbYQ',
        documents: {
          domain: {
            indices: [
              { unique: true, properties: [{ nameHash: 'asc' }] },
              {
                properties: [
                  { normalizedParentDomainName: 'asc' },
                  { normalizedLabel: 'asc' },
                ],
              },
              { properties: [{ 'records.dashIdentity': 'asc' }] },
            ],
            required: [
              'nameHash',
              'label',
              'normalizedLabel',
              'normalizedParentDomainName',
              'preorderSalt',
              'records',
            ],
            properties: {
              label: {
                type: 'string',
                pattern: '^((?!-)[a-zA-Z0-9-]{0,62}[a-zA-Z0-9])$',
                maxLength: 63,
                description: "Domain label. e.g. 'UseR'",
              },
              records: {
                type: 'object',
                properties: {
                  dashIdentity: {
                    type: 'string',
                    pattern: '^[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]+$',
                    maxLength: 44,
                    minLength: 42,
                    description: 'base58 identity id string',
                  },
                },
                minProperties: 1,
                additionalProperties: false,
              },
              nameHash: {
                type: 'string',
                pattern: '^[A-Fa-f0-9]+$',
                maxLength: 68,
                minLength: 68,
                description: 'Double sha-256 multihash of the full domain name in a form of a hex string',
              },
              preorderSalt: {
                type: 'string',
                pattern: '^[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]+$',
                maxLength: 34,
                minLength: 25,
                description: 'Domain pre-order salt. Currently randomly generated base58 address string.',
              },
              normalizedLabel: {
                type: 'string',
                pattern: '^((?!-)[a-z0-9-]{0,62}[a-z0-9])$',
                maxLength: 63,
                description: "Domain label in a lower case. e.g. 'user'",
              },
              normalizedParentDomainName: {
                type: 'string',
                maxLength: 190,
                minLength: 0,
                description: "A full parent domain name in lower case. e.g. 'dash.org'",
              },
            },
            additionalProperties: false,
          },
          preorder: {
            indices: [
              { unique: true, properties: [{ saltedDomainHash: 'asc' }] },
            ],
            required: ['saltedDomainHash'],
            properties: {
              saltedDomainHash: {
                type: 'string',
                pattern: '^[A-Fa-f0-9]+$',
                maxLength: 68,
                minLength: 68,
                description: 'Double sha-256 multihash of the full domain name + salt in a form of a hex string',
              },
            },
            additionalProperties: false,
          },
        },
      });

      stateRepositoryMock.fetchDataContract.withArgs('295xRRRMGYyAruG39XdAibaU9jMAzxhknkkAxFE7uVkW').resolves(
        contract,
      );

      const privateKey = new PrivateKey('a891a0cfbc1235fc768f23af45f8a786ff98b54e263d68d87074be3e8f7c1a02');

      const identity = new Identity({
        id: 'Di94QuVennkE4FfTQHJF2MJhqehLbmodSn5Uzy5u4zHL',
        publicKeys: [
          {
            id: 0,
            type: 0,
            data: privateKey.toPublicKey().toBuffer().toString('base64'),
            isEnabled: true,
          },
        ],
        balance: 9999826,
      });

      stateRepositoryMock.fetchIdentity.withArgs('Di94QuVennkE4FfTQHJF2MJhqehLbmodSn5Uzy5u4zHL').resolves(
        identity,
      );

      const stFromPlayground = new DocumentsBatchTransition(st);

      const pubKey = identity.getPublicKeyById(0);

      stFromPlayground.sign(
        pubKey,
        privateKey,
      );

      const contractValidationResult = await dpp.dataContract.validate(contract);

      expect(contractValidationResult.isValid()).to.be.true();

      const result = await dpp.stateTransition.validateStructure(stFromPlayground);

      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateStructure', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateStructure(
          dataContractCreateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateStructure(
        dataContractCreateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateData', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateData(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateData(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate raw state transition data', async () => {
      const rawStateTransition = dataContractCreateTransition.toJSON();

      const result = await dpp.stateTransition.validateData(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateFee', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateFee(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateFee(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate raw state transition data', async () => {
      const rawStateTransition = dataContractCreateTransition.toJSON();

      const result = await dpp.stateTransition.validateFee(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
