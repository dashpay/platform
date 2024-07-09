const { expect } = require('chai');
const crypto = require('crypto');
const {
  DashPlatformProtocol,
  JsonSchemaError,
} = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const schema = require('../../schema/v1/dashpay.schema.json');

const whitepaperMasternodeText = 'Full nodes are servers running on a P2P network that allow peers to use them to receive updates about the events on the network. These nodes utilize significant amounts of traffic and other resources that incur a substantial cost. As a result, a steady decrease in the amount of these nodes has been observed for some time on the Bitcoin network and as a result, block propagation times have been upwards of 40 seconds. Many solutions have been proposed such as a new reward scheme by Microsoft Research and the Bitnodes incentive program';
const encoded32Chars = '4fafc98bbfe597f7ba2c9f767d52036d';
const encoded64Chars = '4fafc98bbfe597f7ba2c9f767d52036d2226175960a908e355e5c575711eb166';

const expectJsonSchemaError = (validationResult, count = 1, index = 0) => {
  const errors = validationResult.getErrors();
  expect(errors)
    .to
    .have
    .length(count);

  const error = validationResult.getErrors()[index];
  expect(error)
    .to
    .be
    .instanceof(JsonSchemaError);

  return error;
};

describe('Dashpay Contract', () => {
  let dpp;
  let contract;
  let identityId;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
    );

    identityId = await generateRandomIdentifier();
    contract = dpp.dataContract.create(identityId, BigInt(1), schema);
  });

  it('should have a valid contract definition', async () => {
    expect(() => dpp.dataContract.create(identityId, BigInt(1), schema))
      .to
      .not
      .throw();
  });

  describe('Documents', () => {
    describe('Profile', () => {
      let profileData;

      beforeEach(() => {
        profileData = {
          displayName: 'Bob',
        };
      });

      describe('displayName', () => {
        it('should have less than 25 chars length', async () => {
          profileData.displayName = 'AliceAndBobAndCarolAndDanAndEveAndFrankAndIvanAndMikeAndWalterAndWendy';

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxLength');
          expect(error.instancePath)
            .to
            .deep
            .equal('/displayName');
        });
      });

      describe('publicMessage', () => {
        it('should have less than 256 chars length', async () => {
          profileData.publicMessage = whitepaperMasternodeText;

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxLength');
          expect(error.instancePath)
            .to
            .deep
            .equal('/publicMessage');
        });
      });

      describe('avatarUrl', () => {
        it('should not be empty', async () => {
          profileData = {
            avatarUrl: '',
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('format');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarUrl');
        });

        it('should have less than 2048 chars length', async () => {
          profileData = {
            avatarUrl: `https://github.com/dashpay/dash/wiki/Whitepaper?text=${encodeURI(whitepaperMasternodeText)}${encodeURI(whitepaperMasternodeText)}${encodeURI(whitepaperMasternodeText)}${encodeURI(whitepaperMasternodeText)}${encodeURI(whitepaperMasternodeText)}`,
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxLength');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarUrl');
        });

        it('should be of type URL', async () => {
          profileData = {
            avatarUrl: 'notAUrl',
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('format');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarUrl');
        });

        it('should be present if other avatar properties are present', async () => {
          profileData = {
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          let error = expectJsonSchemaError(validationResult, 2, 0);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarUrl' });

          error = expectJsonSchemaError(validationResult, 2, 1);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarUrl' });
        });
      });

      describe('avatarHash', () => {
        it('should have minimum length of 32', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(0),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarHash');
        });

        it('should have maximum length of 32', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(33),
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarHash');
        });

        it('should be of type array', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: 'notAnArray',
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarHash');
        });

        it('should be present if other avatar properties are present', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarFingerprint: Buffer.alloc(8),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          let error = expectJsonSchemaError(validationResult, 2, 0);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarHash' });

          error = expectJsonSchemaError(validationResult, 2, 1);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarHash' });
        });
      });

      describe('avatarFingerprint', () => {
        it('should have minimum length of 8', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(0),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarFingerprint');
        });

        it('should have maximum length of 8', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: Buffer.alloc(33),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarFingerprint');
        });

        it('should be of type array', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(32),
            avatarFingerprint: 'notAnArray',
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.instancePath)
            .to
            .deep
            .equal('/avatarFingerprint');
        });

        it('should be present if other avatar properties are present', async () => {
          profileData = {
            avatarUrl: 'https://github.com/dashpay/dash.jpg',
            avatarHash: Buffer.alloc(32),
          };

          const document = dpp.document.create(contract, identityId, 'profile', profileData);
          const validationResult = document.validate(dpp.protocolVersion);
          let error = expectJsonSchemaError(validationResult, 2, 0);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarFingerprint' });

          error = expectJsonSchemaError(validationResult, 2, 1);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params)
            .to
            .deep
            .equal({ missingProperty: 'avatarFingerprint' });
        });
      });

      it('should not have additional properties', async () => {
        profileData.someOtherProperty = 42;

        const document = dpp.document.create(contract, identityId, 'profile', profileData);
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

      it('at least one of avatarUrl, publicMessage or displayName should be present', async () => {
        const profile = dpp.document.create(contract, identityId, 'profile', { });

        const result = profile.validate(dpp.protocolVersion);

        const error = expectJsonSchemaError(result);

        expect(error.keyword)
          .to
          .equal('minProperties');
      });

      it('should be valid', async () => {
        const profile = dpp.document.create(contract, identityId, 'profile', profileData);

        const result = profile.validate(dpp.protocolVersion);

        expect(result.isValid())
          .to
          .be
          .true();
      });
    });

    describe('Contact info', () => {
      let contactInfoData;

      beforeEach(() => {
        contactInfoData = {
          encToUserId: Buffer.alloc(32),
          privateData: Buffer.alloc(48),
          rootEncryptionKeyIndex: 0,
          derivationEncryptionKeyIndex: 0,
        };
      });

      describe('encToUserId', () => {
        it('should be defined', async () => {
          delete contactInfoData.encToUserId;

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('encToUserId');
        });

        it('should have exactly 32 chars length', async () => {
          contactInfoData.encToUserId = Buffer.from(`${encoded64Chars}11`, 'hex');

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .equal('/encToUserId');
        });

        it('should have more or 32 chars length', async () => {
          contactInfoData.encToUserId = Buffer.from(encoded32Chars, 'hex');

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .equal('/encToUserId');
        });
      });

      describe('rootEncryptionKeyIndex', () => {
        it('should be defined', async () => {
          delete contactInfoData.rootEncryptionKeyIndex;

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('rootEncryptionKeyIndex');
        });

        it('should not be less than 0', async () => {
          contactInfoData.rootEncryptionKeyIndex = -1;

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.instancePath)
            .to
            .equal('/rootEncryptionKeyIndex');
        });
      });

      describe('privateData', () => {
        it('should be defined', async () => {
          delete contactInfoData.privateData;

          const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('privateData');
        });
      });

      it('should not have additional properties', async () => {
        contactInfoData.someOtherProperty = 42;

        const document = dpp.document.create(contract, identityId, 'contactInfo', contactInfoData);
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

    describe('Contact Request', () => {
      let contactRequestData;

      beforeEach(() => {
        contactRequestData = {
          toUserId: Buffer.alloc(32),
          encryptedPublicKey: Buffer.alloc(96),
          senderKeyIndex: 0,
          recipientKeyIndex: 0,
          accountReference: 0,
        };
      });

      describe('toUserId', () => {
        it('should be defined', async () => {
          delete contactRequestData.toUserId;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('toUserId');
        });
      });

      describe('encryptedPublicKey', () => {
        it('should be defined', async () => {
          delete contactRequestData.encryptedPublicKey;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('encryptedPublicKey');
        });
      });

      describe('senderKeyIndex', () => {
        it('should be defined', async () => {
          delete contactRequestData.senderKeyIndex;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('senderKeyIndex');
        });

        it('should not be less than 0', async () => {
          contactRequestData.senderKeyIndex = -1;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.instancePath)
            .to
            .equal('/senderKeyIndex');
        });
      });

      describe('recipientKeyIndex', () => {
        it('should be defined', async () => {
          delete contactRequestData.recipientKeyIndex;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('recipientKeyIndex');
        });

        it('should not be less than 0', async () => {
          contactRequestData.recipientKeyIndex = -1;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.instancePath)
            .to
            .equal('/recipientKeyIndex');
        });
      });

      describe('encryptedAccountLabel', () => {
        it('should have minimum length of 48', async () => {
          contactRequestData.encryptedAccountLabel = Buffer.alloc(0);

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/encryptedAccountLabel');
        });

        it('should have maximum length of 80', async () => {
          contactRequestData.encryptedAccountLabel = Buffer.alloc(82);

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/encryptedAccountLabel');
        });

        it('should be of type array', async () => {
          contactRequestData.encryptedAccountLabel = 'notAnArray';

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.instancePath)
            .to
            .deep
            .equal('/encryptedAccountLabel');
        });
      });

      describe('autoAcceptProof', () => {
        it('should have minimum length of 38', async () => {
          contactRequestData.autoAcceptProof = Buffer.alloc(0);

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/autoAcceptProof');
        });

        it('should have maximum length of 102', async () => {
          contactRequestData.autoAcceptProof = Buffer.alloc(104);

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('maxItems');
          expect(error.instancePath)
            .to
            .deep
            .equal('/autoAcceptProof');
        });

        it('should be of type array', async () => {
          contactRequestData.autoAcceptProof = 'notAnArray';

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult, 2);

          expect(error.keyword)
            .to
            .equal('type');
          expect(error.instancePath)
            .to
            .deep
            .equal('/autoAcceptProof');
        });
      });

      describe('accountReference', () => {
        it('should be defined', async () => {
          delete contactRequestData.accountReference;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('required');
          expect(error.params.missingProperty)
            .to
            .equal('accountReference');
        });

        it('should not be less than 0', async () => {
          contactRequestData.accountReference = -1;

          const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
          const validationResult = document.validate(dpp.protocolVersion);
          const error = expectJsonSchemaError(validationResult);

          expect(error.keyword)
            .to
            .equal('minimum');
          expect(error.instancePath)
            .to
            .equal('/accountReference');
        });
      });

      it('should not have additional properties', async () => {
        contactRequestData.someOtherProperty = 42;

        const document = dpp.document.create(contract, identityId, 'contactRequest', contactRequestData);
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
  });
});
