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
    passPercent: 60,
    minimumTurnoutPercent: 10,
    endDate: Date.now() + TWO_WEEKS_MS,
  });

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
    it('should be valid', () => {
      expect(validate(rawProposal()).isValid()).to.be.true();
    });

    it('should not need a url', () => {
      const raw = rawProposal();
      delete raw.url;

      expect(validate(raw).isValid()).to.be.true();
    });

    it('should keep its rule fixed, its text editable for a while and every version', () => {
      const { proposal } = decisionProposalsContractDocumentsSchema;

      expect(proposal.documentsMutable).to.be.true();
      expect(proposal.documentsKeepHistory).to.be.true();
      expect(proposal.canBeDeleted).to.be.false();
      expect(proposal.immutable.filter((entry) => typeof entry === 'string')).to.deep.equal([
        'passPercent',
        'minimumTurnoutPercent',
        'endDate',
      ]);
      expect(proposal.immutable.filter((entry) => typeof entry === 'object')
        .map(({ property }) => property)).to.deep.equal(['title', 'description', 'url']);
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

    ['title', 'description', 'passPercent', 'minimumTurnoutPercent', 'endDate'].forEach((property) => {
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

    // [property, lowest, highest]: a pass threshold of 50 to 80 percent, a turnout of at most
    // half the network
    [['passPercent', 50, 80], ['minimumTurnoutPercent', 0, 50]].forEach(([property, lowest, highest]) => {
      describe(property, () => {
        it(`should accept ${lowest} and ${highest}`, () => {
          [lowest, highest].forEach((value) => {
            const raw = rawProposal();
            raw[property] = value;

            expect(validate(raw).isValid()).to.be.true();
          });
        });

        it(`should refuse ${highest + 1}`, () => {
          const raw = rawProposal();
          raw[property] = highest + 1;

          const error = expectJsonSchemaError(validate(raw));

          expect(error.keyword).to.equal('maximum');
        });

        if (lowest > 0) {
          it(`should refuse ${lowest - 1}`, () => {
            const raw = rawProposal();
            raw[property] = lowest - 1;

            const error = expectJsonSchemaError(validate(raw));

            expect(error.keyword).to.equal('minimum');
          });
        }
      });
    });
  });
});
