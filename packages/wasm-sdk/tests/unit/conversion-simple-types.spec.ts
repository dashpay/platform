import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Simple Type Conversions', () => {
  before(async () => {
    await init();
  });

  describe('IdentityBalanceAndRevision', () => {
    const jsonFixture = {
      balance: 5000000000,
      revision: 42,
    };

    const objectFixture = {
      balance: 5000000000n,
      revision: 42n,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.IdentityBalanceAndRevision.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.IdentityBalanceAndRevision.fromObject(objectFixture);
        expect(result.toObject()).to.deep.equal(objectFixture);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.IdentityBalanceAndRevision.fromJSON(jsonFixture);
        expect(result.balance).to.equal(5000000000n);
        expect(result.revision).to.equal(42n);
      });
    });

    describe('fromObject()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.IdentityBalanceAndRevision.fromObject(objectFixture);
        expect(result.balance).to.equal(5000000000n);
        expect(result.revision).to.equal(42n);
      });
    });
  });

  describe('ProtocolVersionUpgradeState', () => {
    const jsonFixture = {
      currentProtocolVersion: 7,
      nextProtocolVersion: 8,
      voteCount: 100,
    };

    const objectFixture = {
      currentProtocolVersion: 7,
      nextProtocolVersion: 8,
      voteCount: 100n,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.ProtocolVersionUpgradeState.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.ProtocolVersionUpgradeState.fromObject(objectFixture);
        expect(result.toObject()).to.deep.equal(objectFixture);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.ProtocolVersionUpgradeState.fromJSON(jsonFixture);
        expect(result.currentProtocolVersion).to.equal(7);
        expect(result.nextProtocolVersion).to.equal(8);
        expect(result.voteCount).to.equal(100n);
      });
    });

    describe('optional fields', () => {
      it('should handle null optional fields in JSON', () => {
        const fixture = {
          currentProtocolVersion: 7,
          nextProtocolVersion: null,
          voteCount: null,
        };

        const result = sdk.ProtocolVersionUpgradeState.fromJSON(fixture);
        expect(result.currentProtocolVersion).to.equal(7);
        expect(result.nextProtocolVersion).to.be.undefined();
        expect(result.voteCount).to.be.undefined();
      });
    });
  });

  describe('TokenTotalSupply', () => {
    const jsonFixture = {
      totalSupply: 100000000000,
    };

    const objectFixture = {
      totalSupply: 100000000000n,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.TokenTotalSupply.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should produce expected Object', () => {
        const result = sdk.TokenTotalSupply.fromObject(objectFixture);
        expect(result.toObject()).to.deep.equal(objectFixture);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.TokenTotalSupply.fromJSON(jsonFixture);
        expect(result.totalSupply).to.equal(100000000000n);
      });
    });
  });

  describe('QuorumInfo', () => {
    const jsonFixture = {
      quorumHash: 'abcdef0123456789',
      quorumType: 'llmq_devnet',
      memberCount: 3,
      threshold: 2,
      isVerified: true,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.QuorumInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const result = sdk.QuorumInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.QuorumInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.QuorumInfo.fromJSON(jsonFixture);
        expect(result.quorumHash).to.equal('abcdef0123456789');
        expect(result.quorumType).to.equal('llmq_devnet');
        expect(result.memberCount).to.equal(3);
        expect(result.threshold).to.equal(2);
        expect(result.isVerified).to.equal(true);
      });
    });
  });

  describe('CurrentQuorumsInfo', () => {
    const jsonFixture = {
      quorums: [
        {
          quorumHash: 'aabb',
          quorumType: 'llmq_devnet',
          memberCount: 3,
          threshold: 2,
          isVerified: true,
        },
      ],
      height: 99999,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.CurrentQuorumsInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const result = sdk.CurrentQuorumsInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.CurrentQuorumsInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.CurrentQuorumsInfo.fromJSON(jsonFixture);
        expect(result.height).to.equal(99999n);
        expect(result.quorums).to.have.lengthOf(1);
      });
    });
  });

  describe('RegisterDpnsNameResult', () => {
    const testId1 = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
    const testId2 = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';

    const jsonFixture = {
      preorderDocumentId: testId1,
      domainDocumentId: testId2,
      fullDomainName: 'alice.dash',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.RegisterDpnsNameResult.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const result = sdk.RegisterDpnsNameResult.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.RegisterDpnsNameResult.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });

      it('should serialize identifiers as Uint8Array', () => {
        const result = sdk.RegisterDpnsNameResult.fromJSON(jsonFixture);
        const obj = result.toObject();
        expect(obj.preorderDocumentId).to.be.instanceOf(Uint8Array);
        expect(obj.domainDocumentId).to.be.instanceOf(Uint8Array);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.RegisterDpnsNameResult.fromJSON(jsonFixture);
        expect(result.preorderDocumentId.toBase58()).to.equal(testId1);
        expect(result.domainDocumentId.toBase58()).to.equal(testId2);
        expect(result.fullDomainName).to.equal('alice.dash');
      });
    });
  });

  describe('PrefundedSpecializedBalance', () => {
    const testId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

    const jsonFixture = {
      identityId: testId,
      balance: 50000000,
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.PrefundedSpecializedBalance.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const result = sdk.PrefundedSpecializedBalance.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.PrefundedSpecializedBalance.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.PrefundedSpecializedBalance.fromJSON(jsonFixture);
        expect(result.identityId.toBase58()).to.equal(testId);
        expect(result.balance).to.equal(50000000n);
      });
    });
  });

  describe('PathElement', () => {
    const jsonFixture = {
      path: ['key'],
      key: 'AQI=',
      pathBytes: ['AwQ=', 'AQI='],
      value: 'dmFsdWU=',
      valueBytes: 'dmFsdWU=',
      elementType: 'sumItem',
      sum: '9007199254740993',
      referenceTarget: ['BQY='],
      referenceTargetError: null,
    };

    const objectInputFixture = {
      path: ['key'],
      key: new Uint8Array([1, 2]),
      pathBytes: [new Uint8Array([3, 4]), new Uint8Array([1, 2])],
      value: 'dmFsdWU=',
      valueBytes: new Uint8Array([118, 97, 108, 117, 101]),
      elementType: 'sumItem',
      sum: 9007199254740993n,
      referenceTarget: [new Uint8Array([5, 6])],
      referenceTargetError: null,
    };

    const objectOutputFixture = {
      ...objectInputFixture,
      referenceTargetError: undefined,
    };

    describe('toJSON()', () => {
      it('should serialize binary fields as base64 strings', () => {
        const result = sdk.PathElement.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should serialize binary fields as Uint8Array and sum as BigInt', () => {
        const result = sdk.PathElement.fromObject(objectInputFixture);
        expect(result.toObject()).to.deep.equal(objectOutputFixture);
      });
    });

    describe('fromObject()', () => {
      it('should expose typed getters for tree exploration fields', () => {
        const result = sdk.PathElement.fromObject(objectInputFixture);

        expect(result.key).to.deep.equal(new Uint8Array([1, 2]));
        expect(result.pathBytes).to.deep.equal([
          new Uint8Array([3, 4]),
          new Uint8Array([1, 2]),
        ]);
        expect(result.value).to.equal('dmFsdWU=');
        expect(result.valueBytes).to.deep.equal(new Uint8Array([118, 97, 108, 117, 101]));
        expect(result.elementType).to.equal('sumItem');
        expect(result.sum).to.equal(9007199254740993n);
        expect(result.referenceTarget).to.deep.equal([new Uint8Array([5, 6])]);
        expect(result.referenceTargetError).to.be.undefined();
      });
    });
  });

  describe('TokenPriceInfo', () => {
    const testId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

    const jsonFixture = {
      tokenId: testId,
      currentPrice: '1000',
      basePrice: '500',
    };

    describe('toJSON()', () => {
      it('should produce expected JSON', () => {
        const result = sdk.TokenPriceInfo.fromJSON(jsonFixture);
        expect(result.toJSON()).to.deep.equal(jsonFixture);
      });
    });

    describe('toObject()', () => {
      it('should round-trip with stable output', () => {
        const result = sdk.TokenPriceInfo.fromJSON(jsonFixture);
        const obj = result.toObject();
        const obj2 = sdk.TokenPriceInfo.fromObject(obj).toObject();
        expect(obj2).to.deep.equal(obj);
      });
    });

    describe('fromJSON()', () => {
      it('should deserialize and expose getters', () => {
        const result = sdk.TokenPriceInfo.fromJSON(jsonFixture);
        expect(result.tokenId.toBase58()).to.equal(testId);
        expect(result.currentPrice).to.equal('1000');
        expect(result.basePrice).to.equal('500');
      });
    });
  });
});
