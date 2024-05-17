const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');

const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const crypto = require('crypto');
const rewardSharingContractSchema = require('../../schema/v1/masternode-reward-shares-documents.json');

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

describe('Masternode reward shares contract', () => {
  let dpp;
  let contract;
  let identityId;
  let rewardShare;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    contract = dpp.dataContract.create(
      identityId,
      BigInt(1),
      rewardSharingContractSchema,
    );

    rewardShare = {
      payToId: await generateRandomIdentifier(),
      percentage: 500,
    };
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), rewardSharingContractSchema))
      .to
      .not
      .throw();
  });

  describe('payToId', () => {
    it('should be defined', () => {
      delete rewardShare.payToId;

      const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
      const validationResult = document.validate(dpp.protocolVersion);
      const error = expectJsonSchemaError(validationResult);

      expect(error.keyword)
        .to
        .equal('required');
      expect(error.params.missingProperty)
        .to
        .equal('payToId');
    });

    it('should have no less than 32 bytes', () => {
      rewardShare.payToId = Buffer.alloc(31);

      expect(() => dpp.document.create(contract, identityId, 'rewardShare', rewardShare))
        .to
        .throw();
    });

    it('should have no more than 32 bytes', async () => {
      rewardShare.payToId = Buffer.alloc(33);

      expect(() => dpp.document.create(contract, identityId, 'rewardShare', rewardShare))
        .to
        .throw();
    });
  });

  describe('percentage', () => {
    it('should be defined', () => {
      delete rewardShare.percentage;

      const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
      const validationResult = document.validate(dpp.protocolVersion);
      const error = expectJsonSchemaError(validationResult);

      expect(error.keyword)
        .to
        .equal('required');
      expect(error.params.missingProperty)
        .to
        .equal('percentage');
    });

    it('should not be less than 1', () => {
      rewardShare.percentage = 0;

      const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
      const validationResult = document.validate(dpp.protocolVersion);
      const error = expectJsonSchemaError(validationResult);

      expect(error.keyword)
        .to
        .equal('minimum');
      expect(error.instancePath)
        .to
        .equal('/percentage');
    });

    it('should not be more than 10000', () => {
      rewardShare.percentage = 10001;

      const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
      const validationResult = document.validate(dpp.protocolVersion);
      const error = expectJsonSchemaError(validationResult);

      expect(error.keyword)
        .to
        .equal('maximum');
      expect(error.instancePath)
        .to
        .equal('/percentage');
    });

    it('should be a number', () => {
      rewardShare.percentage = '10';

      const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
      const validationResult = document.validate(dpp.protocolVersion);
      const error = expectJsonSchemaError(validationResult);

      expect(error.keyword)
        .to
        .equal('type');
      expect(error.instancePath)
        .to
        .equal('/percentage');
      expect(error.params.type)
        .to
        .equal('integer');
    });
  });

  it('should should not have additional properties', () => {
    rewardShare.someOtherProperty = 42;

    const document = dpp.document.create(contract, identityId, 'rewardShare', rewardShare);
    const validationResult = document.validate(dpp.protocolVersion);
    const error = expectJsonSchemaError(validationResult);

    expect(error.keyword)
      .to
      .equal('additionalProperties');
    expect(error.params.additionalProperties)
      .to
      .deep
      .equal(['someOtherProperty']);
  });
});
