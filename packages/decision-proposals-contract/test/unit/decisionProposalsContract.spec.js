const crypto = require('crypto');

const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const { expect } = require('chai');
const decisionProposalsContractDocumentsSchema = require('../../schema/v1/decision-proposals-contract-documents.json');

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

const TWO_WEEKS_MS = 14 * 24 * 60 * 60 * 1000;

describe('Decision Proposals Contract', () => {
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
      decisionProposalsContractDocumentsSchema,
    );
  });

  const validate = (raw) => {
    const document = dpp.document.create(dataContract, identityId, 'proposal', raw);
    return document.validate(dpp.protocolVersion);
  };

  const rawProposal = () => ({
    title: 'Should evonodes run the new indexer by default?',
    description: 'Yes: the next release turns it on. No: it stays opt-in.',
    url: 'https://www.dash.org/forum/threads/indexer-default.1/',
    supermajorityNumerator: 2,
    supermajorityDenominator: 3,
    minimumVotingPower: 1000,
    endDate: Date.now() + TWO_WEEKS_MS,
  });

  const rawProposalWithShare = () => {
    const raw = rawProposal();
    delete raw.minimumVotingPower;
    raw.minimumShare = {
      numerator: 1,
      denominator: 10,
      rounding: 'up',
    };

    return raw;
  };

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(
      identityId,
      BigInt(1),
      decisionProposalsContractDocumentsSchema,
    ))
      .to
      .not
      .throw();
  });

  it('should have one document type', () => {
    expect(Object.keys(decisionProposalsContractDocumentsSchema)).to.deep.equal(['proposal']);
  });

  describe('proposal', () => {
    it('should be valid with a fixed minimum voting power', () => {
      expect(validate(rawProposal()).isValid()).to.be.true();
    });

    it('should be valid with a minimum share of the network', () => {
      expect(validate(rawProposalWithShare()).isValid()).to.be.true();
    });

    it('should not need a url', () => {
      const raw = rawProposal();
      delete raw.url;

      expect(validate(raw).isValid()).to.be.true();
    });

    it('should be immutable and undeletable, so masternodes vote on what it said when it opened', () => {
      const { proposal } = decisionProposalsContractDocumentsSchema;

      expect(proposal.documentsMutable).to.be.false();
      expect(proposal.canBeDeleted).to.be.false();
    });

    it('should index proposals by end date, by owner and by creation time', () => {
      const { indices } = decisionProposalsContractDocumentsSchema.proposal;

      expect(indices.map(({ name }) => name)).to.deep.equal([
        'byEndDate',
        'byOwner',
        'byCreatedAt',
      ]);
      indices.forEach(({ unique }) => expect(unique).to.be.undefined());
    });

    ['title', 'description', 'supermajorityNumerator', 'supermajorityDenominator', 'endDate'].forEach((property) => {
      it(`should require ${property}`, () => {
        const raw = rawProposal();
        delete raw[property];

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal(property);
      });
    });

    it('should not have additional properties', () => {
      const raw = rawProposal();
      raw.outcome = 'yes';

      const error = expectJsonSchemaError(validate(raw));

      expect(error.keyword).to.equal('additionalProperties');
      expect(error.params.additionalProperties).to.deep.equal(['outcome']);
    });

    describe('title', () => {
      it('should refuse an empty title', () => {
        const raw = rawProposal();
        raw.title = '';

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('minLength');
      });

      it('should accept 128 characters', () => {
        const raw = rawProposal();
        raw.title = 'a'.repeat(128);

        expect(validate(raw).isValid()).to.be.true();
      });

      it('should refuse 129 characters', () => {
        const raw = rawProposal();
        raw.title = 'a'.repeat(129);

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('maxLength');
      });
    });

    describe('description', () => {
      it('should refuse 4097 characters', () => {
        const raw = rawProposal();
        raw.description = 'a'.repeat(4097);

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('maxLength');
      });

      it('should refuse more than 4096 bytes within 4096 characters', () => {
        // 2049 two-byte characters: within `maxLength`, which counts characters, but
        // 4098 bytes, over `maxBytes` (DocumentPropertyMaxBytesExceededError, 10421)
        const raw = rawProposal();
        raw.description = 'é'.repeat(2049);

        const errors = validate(raw).getErrors();

        expect(errors).to.have.length(1);
        expect(errors[0].getCode()).to.equal(10421);
      });

      it('should accept 4096 bytes in two-byte characters', () => {
        const raw = rawProposal();
        raw.description = 'é'.repeat(2048);

        expect(validate(raw).isValid()).to.be.true();
      });
    });

    describe('supermajority', () => {
      ['supermajorityNumerator', 'supermajorityDenominator'].forEach((property) => {
        it(`should refuse a ${property} of 0`, () => {
          const raw = rawProposal();
          raw[property] = 0;

          const error = expectJsonSchemaError(validate(raw));

          expect(error.keyword).to.equal('minimum');
        });

        it(`should refuse a ${property} over 255`, () => {
          const raw = rawProposal();
          raw[property] = 256;

          const error = expectJsonSchemaError(validate(raw));

          expect(error.keyword).to.equal('maximum');
        });
      });
    });

    describe('minimumVotingPower', () => {
      it('should refuse a value over u32', () => {
        const raw = rawProposal();
        raw.minimumVotingPower = 4294967296;

        expect(validate(raw).isValid()).to.be.false();
      });
    });

    describe('minimumShare', () => {
      ['down', 'up', 'downPlusOne', 'upMinusOne'].forEach((rounding) => {
        it(`should accept the rounding ${rounding}`, () => {
          const raw = rawProposalWithShare();
          raw.minimumShare.rounding = rounding;

          expect(validate(raw).isValid()).to.be.true();
        });
      });

      it('should refuse an unknown rounding', () => {
        const raw = rawProposalWithShare();
        raw.minimumShare.rounding = 'nearest';

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('enum');
      });

      ['numerator', 'denominator', 'rounding'].forEach((property) => {
        it(`should require ${property}`, () => {
          const raw = rawProposalWithShare();
          delete raw.minimumShare[property];

          const error = expectJsonSchemaError(validate(raw));

          expect(error.keyword).to.equal('required');
          expect(error.params.missingProperty).to.equal(property);
        });
      });

      it('should refuse a denominator of 0', () => {
        const raw = rawProposalWithShare();
        raw.minimumShare.denominator = 0;

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('minimum');
      });

      it('should not have additional properties', () => {
        const raw = rawProposalWithShare();
        raw.minimumShare.total = 3000;

        const error = expectJsonSchemaError(validate(raw));

        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperties).to.deep.equal(['total']);
      });
    });
  });
});
