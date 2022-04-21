import 'mocha';
import {expect} from 'chai';
import {acceptContactRequest} from "./acceptContactRequest";
import sinon from 'sinon';
import DataContract from '@dashevo/dpp/lib/dataContract/DataContract'
import Document from '@dashevo/dpp/lib/document/Document'

describe('DashPayPlugin - acceptContactRequest', () => {
  let storage;
  let platform;
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

  const labUserDocument = new Document({
    '$protocolVersion': 0,
    '$id': 'Gedv9XQbginmCN3rVLDoKLt63bT7orBxiCZgNhvTXcXy',
    '$type': 'domain',
    '$dataContractId': 'DRwR6AwqxUKfC1ux6kaBeo2F2YcQRQ1GaiVEv3P5y5BP',
    '$ownerId': 'GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9',
    '$revision': 1,
    label: 'labUser',
    records: {
      dashUniqueIdentityId: 'GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9'
    },
    preorderSalt: '5CHNp0yBs8M+MGdkhdjT/ZB1He+3P1sYbSt3pd/fcbQ=',
    subdomainRules: {allowSubdomains: false},
    normalizedLabel: 'labuser',
    normalizedParentDomainName: 'dash'
  }, dpnsDataContract);


  let sendContactRequest;
  before(function () {
    sendContactRequest = sinonSandbox.stub().usingPromise(Promise).returns(true);
    storage = {
      getIndexedIdentityIds: sinonSandbox.stub().returns(['GzggcEzz9fALyv4R9MuCDaMpXJ8HWMANSHCphPn2hhd9'])
    }
    platform = {
      names: {
        resolveByRecord: sinonSandbox.stub().usingPromise(Promise).returns([labUserDocument])
      },
    }
  })
  it('should accept a contact request', async function () {
    const self = {
      walletId: 'squawk7700',
      storage,
      platform,
      sendContactRequest,
    };

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

    //@ts-ignore
    const acceptContactRequestResult = await acceptContactRequest.call(self, contactReqDocument);
    expect(platform.names.resolveByRecord.callCount).to.equal(1);
    expect(sendContactRequest.callCount).to.equal(1);
    expect(sendContactRequest.firstCall.args).to.deep.equal(['labUser']);
    expect(acceptContactRequestResult).to.equal(true);
  });

})
;
