import 'mocha';
import {expect} from 'chai';
import {sendContactRequest} from "./sendContactRequest";
import {encryptSharedKey} from "./encryptSharedKey";
import {createAccountReference} from "./createAccountReference";
import {encryptPublicKey} from "./encryptPublicKey";
import {encryptAccountLabel} from "./encryptAccountLabel";
import sinon from 'sinon';
import Identity from '@dashevo/dpp/lib/identity/Identity'
import DataContract from '@dashevo/dpp/lib/dataContract/DataContract'
import Document from '@dashevo/dpp/lib/document/Document'
import exp from "constants";
import {HDPrivateKey, PrivateKey} from "@dashevo/dashcore-lib";
import {plugins} from "@dashevo/wallet-lib";

describe('DashPayPlugin - sendContactRequest', () => {
  let storage;
  let platform;
  let identities;
  let keyChain;
  let sinonSandbox = sinon.createSandbox();
  const dashpayDataContract = new DataContract({
      protocolVersion: 0,
      '$id': 'B5kbZtUfzuPVH3MUcmQpUcZTFZBf7nfEZd1bmRDD7km6',
      '$schema': 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
      ownerId: 'EzvGUTe9CQoogizPZRYeta91bvowNMZ6mdT7ZLjbQ7X5',
      documents: {
        profile: {
          type: 'object',
          indices: [
            {unique: true, properties: [{'$ownerId': 'asc'}]},
            {
              properties: [{'$ownerId': 'asc'}, {'$updatedAt': 'asc'}]
            }
          ],
          required: ['$createdAt', '$updatedAt'],
          properties: {
            avatarUrl: {type: 'string', format: 'url', maxLength: 2048},
            avatarHash: {
              type: 'array',
              maxItems: 32,
              minItems: 32,
              byteArray: true,
              description: 'SHA256 hash of the bytes of the image specified by avatarUrl'
            },
            displayName: {type: 'string', maxLength: 25},
            publicMessage: {type: 'string', maxLength: 140},
            avatarFingerprint: {
              type: 'array',
              maxItems: 8,
              minItems: 8,
              byteArray: true,
              description: 'dHash the image specified by avatarUrl'
            }
          },
          additionalProperties: false
        },
        contactInfo: {
          type: 'object',
          indices: [
            {
              unique: true,
              properties: [
                {'$ownerId': 'asc'},
                {rootEncryptionKeyIndex: 'asc'},
                {derivationEncryptionKeyIndex: 'asc'}
              ]
            },
            {
              properties: [{'$ownerId': 'asc'}, {'$updatedAt': 'asc'}]
            }
          ],
          required: [
            '$createdAt',
            '$updatedAt',
            'encToUserId',
            'privateData',
            'rootEncryptionKeyIndex',
            'derivationEncryptionKeyIndex'
          ],
          properties: {
            encToUserId: {type: 'array', maxItems: 32, minItems: 32, byteArray: true},
            privateData: {
              type: 'array',
              maxItems: 2048,
              minItems: 48,
              byteArray: true,
              description: 'This is the encrypted values of aliasName + note + displayHidden encoded as an array in cbor'
            },
            rootEncryptionKeyIndex: {type: 'integer', minimum: 0},
            derivationEncryptionKeyIndex: {type: 'integer', minimum: 0}
          },
          additionalProperties: false
        },
        contactRequest: {
          type: 'object',
          indices: [
            {
              unique: true,
              properties: [
                {'$ownerId': 'asc'},
                {toUserId: 'asc'},
                {accountReference: 'asc'}
              ]
            },
            {properties: [{'$ownerId': 'asc'}, {toUserId: 'asc'}]},
            {
              properties: [{toUserId: 'asc'}, {'$createdAt': 'asc'}]
            },
            {
              properties: [{'$ownerId': 'asc'}, {'$createdAt': 'asc'}]
            }
          ],
          required: [
            '$createdAt',
            'toUserId',
            'encryptedPublicKey',
            'senderKeyIndex',
            'recipientKeyIndex',
            'accountReference'
          ],
          properties: {
            toUserId: {
              type: 'array',
              maxItems: 32,
              minItems: 32,
              byteArray: true,
              contentMediaType: 'application/x.dash.dpp.identifier'
            },
            senderKeyIndex: {type: 'integer', minimum: 0},
            autoAcceptProof: {type: 'array', maxItems: 102, minItems: 38, byteArray: true},
            accountReference: {type: 'integer', minimum: 0},
            recipientKeyIndex: {type: 'integer', minimum: 0},
            encryptedPublicKey: {type: 'array', maxItems: 96, minItems: 96, byteArray: true},
            coreHeightCreatedAt: {type: 'integer', minimum: 1},
            encryptedAccountLabel: {type: 'array', maxItems: 80, minItems: 48, byteArray: true}
          },
          additionalProperties: false
        }
      }
    }
  );
  const dpnsDataContract = new DataContract({
      protocolVersion: 0,
      '$id': 'DRwR6AwqxUKfC1ux6kaBeo2F2YcQRQ1GaiVEv3P5y5BP',
      '$schema': 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
      ownerId: 'HnLZjtrhqZsMgPUeyQa9CxQX1nkKjJFcc3cpytRikUwg',
      documents: {
        domain: {
          type: 'object',
          indices: [
            {
              unique: true,
              properties: [
                {normalizedParentDomainName: 'asc'},
                {normalizedLabel: 'asc'}
              ]
            },
            {
              unique: true,
              properties: [{'records.dashUniqueIdentityId': 'asc'}]
            },
            {properties: [{'records.dashAliasIdentityId': 'asc'}]}
          ],
          '$comment': "In order to register a domain you need to create a preorder. The preorder step is needed to prevent man-in-the-middle attacks. normalizedLabel + '.' + normalizedParentDomain must not be longer than 253 chars length as defined by RFC 1035. Domain documents are immutable: modification and deletion are restricted",
          required: [
            'label',
            'normalizedLabel',
            'normalizedParentDomainName',
            'preorderSalt',
            'records',
            'subdomainRules'
          ],
          properties: {
            label: {
              type: 'string',
              pattern: '^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$',
              maxLength: 63,
              minLength: 3,
              description: "Domain label. e.g. 'Bob'."
            },
            records: {
              type: 'object',
              '$comment': 'Constraint with max and min properties ensure that only one identity record is used - either a `dashUniqueIdentityId` or a `dashAliasIdentityId`',
              properties: {
                dashAliasIdentityId: {
                  type: 'array',
                  '$comment': 'Must be equal to the document owner',
                  maxItems: 32,
                  minItems: 32,
                  byteArray: true,
                  description: 'Identity ID to be used to create alias names for the Identity',
                  contentMediaType: 'application/x.dash.dpp.identifier'
                },
                dashUniqueIdentityId: {
                  type: 'array',
                  '$comment': 'Must be equal to the document owner',
                  maxItems: 32,
                  minItems: 32,
                  byteArray: true,
                  description: 'Identity ID to be used to create the primary name the Identity',
                  contentMediaType: 'application/x.dash.dpp.identifier'
                }
              },
              maxProperties: 1,
              minProperties: 1,
              additionalProperties: false
            },
            preorderSalt: {
              type: 'array',
              maxItems: 32,
              minItems: 32,
              byteArray: true,
              description: 'Salt used in the preorder document'
            },
            subdomainRules: {
              type: 'object',
              required: ['allowSubdomains'],
              properties: {
                allowSubdomains: {
                  type: 'boolean',
                  '$comment': 'Only the domain owner is allowed to create subdomains for non top-level domains',
                  description: 'This option defines who can create subdomains: true - anyone; false - only the domain owner'
                }
              },
              description: 'Subdomain rules allow domain owners to define rules for subdomains',
              additionalProperties: false
            },
            normalizedLabel: {
              type: 'string',
              pattern: '^[a-z0-9][a-z0-9-]{0,61}[a-z0-9]$',
              '$comment': 'Must be equal to the label in lowercase. This property will be deprecated due to case insensitive indices',
              maxLength: 63,
              description: "Domain label in lowercase for case-insensitive uniqueness validation. e.g. 'bob'"
            },
            normalizedParentDomainName: {
              type: 'string',
              pattern: '^$|^[[a-z0-9][a-z0-9-\\.]{0,188}[a-z0-9]$',
              '$comment': 'Must either be equal to an existing domain or empty to create a top level domain. Only the data contract owner can create top level domains.',
              maxLength: 190,
              minLength: 0,
              description: "A full parent domain name in lowercase for case-insensitive uniqueness validation. e.g. 'dash'"
            }
          },
          additionalProperties: false
        },
        preorder: {
          type: 'object',
          indices: [
            {unique: true, properties: [{saltedDomainHash: 'asc'}]}
          ],
          '$comment': 'Preorder documents are immutable: modification and deletion are restricted',
          required: ['saltedDomainHash'],
          properties: {
            saltedDomainHash: {
              type: 'array',
              maxItems: 32,
              minItems: 32,
              byteArray: true,
              description: 'Double sha-256 of the concatenation of a 32 byte random salt and a normalized domain name'
            }
          },
          additionalProperties: false
        }
      }
    }
  );
  const lunchUserDocument = new Document({
    '$protocolVersion': 0,
    '$id': '83NL94UvERCvCTvLspaLSNT6vtZFka6cG2v5Lg7ViixB',
    '$type': 'domain',
    '$dataContractId': 'DRwR6AwqxUKfC1ux6kaBeo2F2YcQRQ1GaiVEv3P5y5BP',
    '$ownerId': '6xn4K2EVQHVEkBqaciRS9YXxUJuswm83FWsqiKTmdPK3',
    '$revision': 1,
    label: 'lunchUser',
    records: {
      dashUniqueIdentityId: '6xn4K2EVQHVEkBqaciRS9YXxUJuswm83FWsqiKTmdPK3'
    },
    preorderSalt: 'KcDDM6ENf06ESUF0IMw0Hq7ECmZjXwUr59qVg9RsuiY=',
    subdomainRules: {allowSubdomains: false},
    normalizedLabel: 'lunchuser',
    normalizedParentDomainName: 'dash'
  }, dpnsDataContract);
  const createdContactRequestDocument = new Document({
    '$protocolVersion': 0,
    '$id': 'EBSZeDqHNiBJjEJsKhXAAGh6cicqSGCkgutLoNtpHnHF',
    '$type': 'contactRequest',
    '$dataContractId': 'B5kbZtUfzuPVH3MUcmQpUcZTFZBf7nfEZd1bmRDD7km6',
    '$ownerId': 'GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9',
    '$revision': 1,
    toUserId: '6xn4K2EVQHVEkBqaciRS9YXxUJuswm83FWsqiKTmdPK3',
    encryptedPublicKey: 'Oqz7iQjVkVMknJImDrCt1x/ydoFSJPhKKZSSAJqFMrDHL8/OZXg/tVCls/iB4a0tZeN0VJbmp3Ga6AB44jIkYRrHekA28PKW/wTxBUDdLIBbvaG04tkhWfNaRty8Bh+k',
    senderKeyIndex: 0,
    recipientKeyIndex: 0,
    accountReference: 93353124,
    encryptedAccountLabel: '9n/ONYc9mgK0kpSgsAWkj7HxDZOrYZ5mpm3wNO8/AcrbCa86Lyxg1lKs749rX7Nw',
    '$createdAt': 1631231150911
  }, dashpayDataContract);
  const identitySender = new Identity({
      protocolVersion: 0,
      id: 'GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9',
      publicKeys: [
        {
          id: 0,
          type: 0,
          data: 'A+oNvgz3xX9W+1QuSE+Wj0te8UdcT2AVDe90Os19/ngK'
        }
      ],
      balance: 9998418,
      revision: 0
    }
  );
  const identityReceiver = new Identity({
      protocolVersion: 0,
      id: '6xn4K2EVQHVEkBqaciRS9YXxUJuswm83FWsqiKTmdPK3',
      publicKeys: [
        {
          id: 0,
          type: 0,
          data: 'A6n088FAn6hNonXv/2/yIDID211ceE1UOobhsvC/TD6P'
        }
      ],
      balance: 9998412,
      revision: 0
    }
  );
  const hdPrivKey = new HDPrivateKey('tprv8pXYkBzWF7XTXdfi92kCMxZBVveSMDik9JAv6nm8MvwPkCbhE2of1jF8m5LuGcGbcYoktG7p1JAZnHq1mfDtiy6GHs9zBhBYEMRcGNFUXRf');
  before(function () {
    storage = {
      getIndexedIdentityIds: sinonSandbox.stub().returns(['GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9'])
    }
    platform = {
      identities: {
        get: sinonSandbox.stub().usingPromise(Promise).onCall(0).returns(identitySender).onCall(1).returns(identityReceiver)
      },
      names: {
        resolve: sinonSandbox.stub().usingPromise(Promise).returns(lunchUserDocument)
      },
      documents: {
        create: sinonSandbox.stub().usingPromise(Promise).returns(createdContactRequestDocument),
        broadcast: sinonSandbox.stub().usingPromise(Promise).returns(true)
      }
    }
    keyChain = {
      getDIP15ExtendedKey: sinonSandbox.stub().returns(hdPrivKey)
    }
    identities = {
      getIdentityHDKeyByIndex: sinonSandbox.stub().returns({privateKey: new PrivateKey('2fc4145c8b7a871c42e32733a83c36f9b0d0eb646f40e53cb9ae0f48669ab0d7')})
    }
  })
  it('should send a contact request', async function () {
    // const sharedSecret = '0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5';
    const self = {
      walletId: 'squawk7700',
      storage,
      identities,
      platform,
      keyChain,
      encryptSharedKey,
      createAccountReference,
      encryptPublicKey,
      encryptAccountLabel
    };
    const accountLabel = 'Default account';
    const contactName = 'Bob';
    const expectedResult = true;
    //@ts-ignore
    const sendContactRequestResult = await sendContactRequest.call(self, contactName, accountLabel);
    expect(storage.getIndexedIdentityIds.callCount).to.equal(1);
    expect(storage.getIndexedIdentityIds.firstCall.args[0]).to.equal(self.walletId);
    expect(storage.getIndexedIdentityIds.firstCall.returnValue).to.deep.equal(['GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9']);

    expect(identities.getIdentityHDKeyByIndex.callCount).to.equal(1);

    expect(platform.identities.get.callCount).to.equal(2);

    expect(platform.names.resolve.callCount).to.equal(1);

    expect(keyChain.getDIP15ExtendedKey.callCount).to.equal(1);

    expect(platform.documents.create.callCount).to.equal(1);

    expect(platform.documents.broadcast.callCount).to.equal(1);

    const contactReqDocument = new Document({
        '$protocolVersion': 0,
        '$id': 'EBSZeDqHNiBJjEJsKhXAAGh6cicqSGCkgutLoNtpHnHF',
        '$type': 'contactRequest',
        '$dataContractId': 'B5kbZtUfzuPVH3MUcmQpUcZTFZBf7nfEZd1bmRDD7km6',
        '$ownerId': 'GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9',
        '$revision': 1,
        toUserId: '6xn4K2EVQHVEkBqaciRS9YXxUJuswm83FWsqiKTmdPK3',
        encryptedPublicKey: 'Oqz7iQjVkVMknJImDrCt1x/ydoFSJPhKKZSSAJqFMrDHL8/OZXg/tVCls/iB4a0tZeN0VJbmp3Ga6AB44jIkYRrHekA28PKW/wTxBUDdLIBbvaG04tkhWfNaRty8Bh+k',
        senderKeyIndex: 0,
        recipientKeyIndex: 0,
        accountReference: 93353124,
        encryptedAccountLabel: '9n/ONYc9mgK0kpSgsAWkj7HxDZOrYZ5mpm3wNO8/AcrbCa86Lyxg1lKs749rX7Nw',
        '$createdAt': 1631231150911
      }
      , dashpayDataContract);
    const batch = {
      create: [contactReqDocument],
      replace: [],
      delete: []
    };
    expect(platform.documents.broadcast.firstCall.args).to.deep.equal([batch,identitySender]);
    expect(sendContactRequestResult).to.deep.equal(expectedResult);
  });

})
;
