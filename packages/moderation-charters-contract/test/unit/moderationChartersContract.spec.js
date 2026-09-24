const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const moderationChartersContractDocumentsSchema = require('../../schema/v1/moderation-charters-contract-documents.json');

const expectJsonSchemaError = (validationResult, errorCount = 1) => {
  const errors = validationResult.getErrors();
  expect(errors)
    .to
    .have
    .length(errorCount);

  const error = validationResult.getErrors()[0];
  expect(error)
    .to
    .be
    .instanceof(JsonSchemaError);

  return error;
};

const randomIds = (count) => Array.from({ length: count }, () => crypto.randomBytes(32));

describe('Moderation Charters Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    dataContract = dpp.dataContract.create(
      identityId,
      BigInt(1),
      moderationChartersContractDocumentsSchema,
    );
  });

  const validate = (type, raw) => {
    const document = dpp.document.create(dataContract, identityId, type, raw);
    return document.validate(dpp.protocolVersion);
  };

  const expectRequired = (type, rawFactory, properties) => {
    properties.forEach((property) => {
      it(`should require ${property}`, async () => {
        const raw = await rawFactory();
        delete raw[property];

        const error = expectJsonSchemaError(validate(type, raw));

        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal(property);
      });
    });
  };

  const expectNoAdditionalProperties = (type, rawFactory, extra) => {
    it(`should not have additional properties such as ${extra}`, async () => {
      const raw = await rawFactory();
      raw[extra] = 42;

      const error = expectJsonSchemaError(validate(type, raw));

      expect(error.keyword).to.equal('additionalProperties');
      expect(error.params.additionalProperties).to.deep.equal([extra]);
    });
  };

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(
      identityId,
      BigInt(1),
      moderationChartersContractDocumentsSchema,
    ))
      .to
      .not
      .throw();
  });

  it('should have seven document types', () => {
    expect(Object.keys(moderationChartersContractDocumentsSchema).sort()).to.deep.equal([
      'addedModerator',
      'electedCharter',
      'joinRequest',
      'reason',
      'removedModerator',
      'resignationRequest',
      'submittedCharter',
    ]);
  });

  describe('reason', () => {
    const rawReason = async () => ({
      code: 'SPM',
      label: 'Spam',
      description: 'Unsolicited promotion, repeated or automated.',
    });

    it('should be valid', async () => {
      expect(validate('reason', await rawReason()).isValid()).to.be.true();
    });

    it('should not need a description', async () => {
      const raw = await rawReason();
      delete raw.description;

      expect(validate('reason', raw).isValid()).to.be.true();
    });

    expectRequired('reason', rawReason, ['code', 'label']);
    expectNoAdditionalProperties('reason', rawReason, 'severity');

    ['spm', 'SP', 'SPAM', 'SP1'].forEach((code) => {
      it(`should refuse the code ${code}`, async () => {
        const raw = await rawReason();
        raw.code = code;

        expect(validate('reason', raw).isValid()).to.be.false();
      });
    });

    it('should refuse a label over 64 characters', async () => {
      const raw = await rawReason();
      raw.label = 'a'.repeat(65);

      const error = expectJsonSchemaError(validate('reason', raw));

      expect(error.keyword).to.equal('maxLength');
    });
  });

  describe('submittedCharter', () => {
    const rawProposal = async () => ({
      targetContractId: await generateRandomIdentifier(),
      description: 'We remove spam and doxing within a day and warn before we act.',
      reasons: randomIds(2),
      moderatorsShare: 60,
      rewardSplit: {
        leader: 10,
        equal: 40,
        actions: 50,
      },
    });

    it('should be valid', async () => {
      expect(validate('submittedCharter', await rawProposal()).isValid()).to.be.true();
    });

    expectRequired('submittedCharter', rawProposal, ['targetContractId', 'description', 'reasons', 'rewardSplit']);
    expectNoAdditionalProperties('submittedCharter', rawProposal, 'members');
    expectNoAdditionalProperties('submittedCharter', rawProposal, 'abilities');

    describe('description', () => {
      it('should refuse 4097 characters', async () => {
        const raw = await rawProposal();
        raw.description = 'a'.repeat(4097);

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('maxLength');
      });

      it('should refuse more than 4096 bytes within 4096 characters', async () => {
        // 2049 two-byte characters: within `maxLength`, which counts characters, but
        // 4098 bytes, over `maxBytes` (DocumentPropertyMaxBytesExceededError, 10421)
        const raw = await rawProposal();
        raw.description = 'é'.repeat(2049);

        const errors = validate('submittedCharter', raw).getErrors();

        expect(errors).to.have.length(1);
        expect(errors[0].getCode()).to.equal(10421);
      });

      it('should accept 4096 bytes in two-byte characters', async () => {
        const raw = await rawProposal();
        raw.description = 'é'.repeat(2048);

        expect(validate('submittedCharter', raw).isValid()).to.be.true();
      });
    });

    describe('reasons', () => {
      it('should accept none, a team that can take no action', async () => {
        const raw = await rawProposal();
        raw.reasons = [];

        expect(validate('submittedCharter', raw).isValid()).to.be.true();
      });

      it('should accept sixty-four', async () => {
        const raw = await rawProposal();
        raw.reasons = randomIds(64);

        expect(validate('submittedCharter', raw).isValid()).to.be.true();
      });

      it('should refuse sixty-five', async () => {
        const raw = await rawProposal();
        raw.reasons = randomIds(65);

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('maxItems');
      });

      it('should refuse a repeated reason', async () => {
        const raw = await rawProposal();
        const [reason] = randomIds(1);
        raw.reasons = [reason, Buffer.from(reason)];

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('uniqueItems');
      });
    });

    describe('moderatorsShare', () => {
      it('should be optional, the full declared fee', async () => {
        const raw = await rawProposal();
        delete raw.moderatorsShare;

        expect(validate('submittedCharter', raw).isValid()).to.be.true();
      });

      it('should accept 0, a team that takes no rewards', async () => {
        const raw = await rawProposal();
        raw.moderatorsShare = 0;

        expect(validate('submittedCharter', raw).isValid()).to.be.true();
      });

      it('should refuse a share over 100', async () => {
        const raw = await rawProposal();
        raw.moderatorsShare = 101;

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('maximum');
      });
    });

    describe('rewardSplit', () => {
      ['leader', 'equal', 'actions'].forEach((share) => {
        it(`should require ${share}`, async () => {
          const raw = await rawProposal();
          delete raw.rewardSplit[share];

          const error = expectJsonSchemaError(validate('submittedCharter', raw));

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal(share);
        });
      });

      it('should refuse a share over 100', async () => {
        const raw = await rawProposal();
        raw.rewardSplit.leader = 101;

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('maximum');
      });

      it('should refuse an unknown share', async () => {
        const raw = await rawProposal();
        raw.rewardSplit.bonus = 0;

        const error = expectJsonSchemaError(validate('submittedCharter', raw));

        expect(error.keyword).to.equal('additionalProperties');
      });
    });
  });

  describe('joinRequest', () => {
    const rawJoinRequest = async () => ({
      submittedCharterId: await generateRandomIdentifier(),
      recipientId: await generateRandomIdentifier(),
      recipientKeyId: 3,
      senderKeyId: 2,
      // A 16-byte IV and two AES blocks.
      encryptedMessage: crypto.randomBytes(48),
    });

    it('should be valid', async () => {
      expect(validate('joinRequest', await rawJoinRequest()).isValid()).to.be.true();
    });

    expectRequired('joinRequest', rawJoinRequest, ['submittedCharterId', 'recipientId', 'recipientKeyId', 'senderKeyId', 'encryptedMessage']);
    expectNoAdditionalProperties('joinRequest', rawJoinRequest, 'message');

    it('should refuse a message shorter than an IV and a block', async () => {
      const raw = await rawJoinRequest();
      raw.encryptedMessage = crypto.randomBytes(31);

      const error = expectJsonSchemaError(validate('joinRequest', raw));

      expect(error.keyword).to.equal('minItems');
    });

    it('should refuse a message over 1040 bytes', async () => {
      const raw = await rawJoinRequest();
      raw.encryptedMessage = crypto.randomBytes(1041);

      const error = expectJsonSchemaError(validate('joinRequest', raw));

      expect(error.keyword).to.equal('maxItems');
    });

    it('should refuse a key id over u32', async () => {
      const raw = await rawJoinRequest();
      raw.senderKeyId = 4294967296;

      expect(validate('joinRequest', raw).isValid()).to.be.false();
    });
  });

  describe('electedCharter', () => {
    const rawElectedCharter = async () => ({
      targetContractId: await generateRandomIdentifier(),
      submittedCharterId: await generateRandomIdentifier(),
      members: randomIds(2),
    });

    it('should be valid', async () => {
      expect(validate('electedCharter', await rawElectedCharter()).isValid()).to.be.true();
    });

    expectRequired('electedCharter', rawElectedCharter, ['targetContractId', 'submittedCharterId', 'members']);
    expectNoAdditionalProperties('electedCharter', rawElectedCharter, 'leaderPower');

    it('should accept a leader running alone', async () => {
      const raw = await rawElectedCharter();
      raw.members = [];

      expect(validate('electedCharter', raw).isValid()).to.be.true();
    });

    it('should accept fifteen members', async () => {
      const raw = await rawElectedCharter();
      raw.members = randomIds(15);

      expect(validate('electedCharter', raw).isValid()).to.be.true();
    });

    it('should refuse sixteen members', async () => {
      const raw = await rawElectedCharter();
      raw.members = randomIds(16);

      const error = expectJsonSchemaError(validate('electedCharter', raw));

      expect(error.keyword).to.equal('maxItems');
    });

    it('should refuse a repeated member', async () => {
      const raw = await rawElectedCharter();
      const [member] = randomIds(1);
      raw.members = [member, Buffer.from(member)];

      const error = expectJsonSchemaError(validate('electedCharter', raw));

      expect(error.keyword).to.equal('uniqueItems');
    });
  });

  describe('addedModerator', () => {
    const rawAddition = async () => ({
      electedCharterId: await generateRandomIdentifier(),
      submittedCharterId: await generateRandomIdentifier(),
      memberId: await generateRandomIdentifier(),
    });

    it('should be valid', async () => {
      expect(validate('addedModerator', await rawAddition()).isValid()).to.be.true();
    });

    it('should be deletable, which takes the member off the team', () => {
      expect(moderationChartersContractDocumentsSchema.addedModerator.canBeDeleted).to.be.true();
    });

    expectRequired('addedModerator', rawAddition, ['electedCharterId', 'submittedCharterId', 'memberId']);
    expectNoAdditionalProperties('addedModerator', rawAddition, 'power');
  });

  describe('removedModerator', () => {
    const rawRemoval = async () => ({
      electedCharterId: await generateRandomIdentifier(),
      memberId: await generateRandomIdentifier(),
    });

    it('should be valid without a resignation', async () => {
      expect(validate('removedModerator', await rawRemoval()).isValid()).to.be.true();
    });

    it('should be deletable, which puts the member back on the team', () => {
      expect(moderationChartersContractDocumentsSchema.removedModerator.canBeDeleted).to.be.true();
    });

    it('should remove only an elected member', () => {
      const { refersTo } = moderationChartersContractDocumentsSchema.removedModerator.properties.memberId;

      expect(refersTo).to.deep.equal({
        type: 'listElement',
        documentType: 'electedCharter',
        propertyAgreement: { electedCharterId: '$id' },
        inList: 'members',
      });
    });

    expectRequired('removedModerator', rawRemoval, ['electedCharterId', 'memberId']);
    expectNoAdditionalProperties('removedModerator', rawRemoval, 'resignationRequestId');
  });

  describe('resignationRequest', () => {
    const rawResignation = async () => ({
      electedCharterId: await generateRandomIdentifier(),
      recipientId: await generateRandomIdentifier(),
      recipientKeyId: 3,
      senderKeyId: 2,
      // A 16-byte IV and two AES blocks.
      encryptedMessage: crypto.randomBytes(48),
    });

    it('should be valid', async () => {
      expect(validate('resignationRequest', await rawResignation()).isValid()).to.be.true();
    });

    it('should be deletable, which withdraws it', () => {
      expect(moderationChartersContractDocumentsSchema.resignationRequest.canBeDeleted).to.be.true();
    });

    it('should let only a team member ask to leave', () => {
      const { anyOf } = moderationChartersContractDocumentsSchema.resignationRequest.ownerRefersTo;

      expect(anyOf.map(({ type, documentType }) => [type, documentType])).to.deep.equal([
        ['listElement', 'electedCharter'],
        ['deletableDocument', 'addedModerator'],
      ]);
    });

    expectRequired('resignationRequest', rawResignation, ['electedCharterId', 'recipientId', 'recipientKeyId', 'senderKeyId', 'encryptedMessage']);
    expectNoAdditionalProperties('resignationRequest', rawResignation, 'memberId');

    it('should refuse a message shorter than an IV and a block', async () => {
      const raw = await rawResignation();
      raw.encryptedMessage = crypto.randomBytes(31);

      const error = expectJsonSchemaError(validate('resignationRequest', raw));

      expect(error.keyword).to.equal('minItems');
    });
  });
});
