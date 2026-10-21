const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const tokenMarketContractDocumentsSchema = require('../../schema/v1/token-market-contract-documents.json');

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

describe('Token Market Contract', () => {
  let dpp;
  let dataContract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();

    // The deployed contract stores every integer as a signed 8-byte value
    // (`sizedIntegerTypes: false`), as a summed property must be: the default
    // config would infer u64 for `remaining` and `escrow`, which a sum cannot
    // hold. `create` takes no config, so the schemas go into the raw form of a
    // placeholder contract whose config is set to match, validated on load.
    const placeholder = dpp.dataContract.create(identityId, BigInt(1), {
      placeholder: {
        type: 'object',
        properties: {
          name: { type: 'string', maxLength: 1, position: 0 },
        },
        additionalProperties: false,
      },
    });
    const rawDataContract = placeholder.toObject();
    rawDataContract.config.sizedIntegerTypes = false;
    rawDataContract.documentSchemas = tokenMarketContractDocumentsSchema;

    dataContract = await dpp.dataContract.createFromObject(rawDataContract);
  });

  const validate = (type, raw) => {
    const document = dpp.document.create(dataContract, identityId, type, raw);
    return document.validate(dpp.protocolVersion);
  };

  const order = (extra = {}) => ({
    tokenId: crypto.randomBytes(32),
    price: 100,
    amount: 1000,
    remaining: 1000,
    minFill: 1,
    ...extra,
  });

  ['ask', 'bid'].forEach((type) => {
    const raw = (extra = {}) => (type === 'bid' ? order({ escrow: 1, ...extra }) : order(extra));

    describe(type, () => {
      it('should accept a valid order', () => {
        expect(validate(type, raw()).isValid()).to.be.true();
      });

      ['tokenId', 'price', 'amount', 'remaining', 'minFill'].forEach((property) => {
        it(`should require ${property}`, () => {
          const document = raw();
          delete document[property];

          const error = expectJsonSchemaError(validate(type, document));

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal(property);
        });
      });

      ['price', 'amount', 'remaining', 'minFill'].forEach((property) => {
        it(`should refuse a ${property} below 1`, () => {
          const error = expectJsonSchemaError(validate(type, raw({ [property]: 0 })));

          expect(error.keyword).to.equal('minimum');
        });
      });

      it('should refuse an unknown property', () => {
        const error = expectJsonSchemaError(validate(type, { ...raw(), note: 'x' }));

        expect(error.keyword).to.equal('additionalProperties');
      });
    });
  });

  it('should require the escrow of a bid', () => {
    const error = expectJsonSchemaError(validate('bid', order()));

    expect(error.keyword).to.equal('required');
    expect(error.params.missingProperty).to.equal('escrow');
  });
});
