/* eslint-disable no-bitwise */
const { expect } = require('chai');
const { BlockHeader } = require('@dashevo/dashcore-lib');

const SpvChain = require('../lib/spvchain');
const consensus = require('../lib/consensus');
const pow = require('../lib/pow');
const utils = require('../lib/utils');
const difficultyCases = require('./data/difficulty');

const toHeaders = (rawHeaders) => rawHeaders.map((raw) => new BlockHeader(Buffer.from(raw, 'hex')));

// toObject() also holds the hash, which must not be copied to a changed header
const withFields = (header, fields) => {
  const { hash, ...object } = header.toObject();
  return new BlockHeader({ ...object, ...fields });
};

const withBits = (header, bits) => withFields(header, { bits });

// A case's headers, with genesis first if they start at block 1
const caseChain = ({ network, height, headers: rawHeaders }) => {
  const headers = toHeaders(rawHeaders);
  const firstHeight = height - headers.length + 1;
  if (firstHeight === 1) {
    return { headers: [new SpvChain(network).genesis, ...headers], firstHeight: 0 };
  }
  return { headers, firstHeight };
};

const findCase = (network, height) => {
  const { headers } = caseChain(
    difficultyCases.find((c) => c.network === network && c.height === height),
  );
  return {
    header: headers[headers.length - 1],
    history: headers.slice(0, -1),
  };
};

// Scales the target, keeping the exponent: difficulty changes by 1 / factor
const scaleTarget = (bits, factor) => {
  const exponent = bits >>> 24;
  const mantissa = Math.round((bits & 0x007fffff) * factor);
  return ((exponent << 24) | mantissa) >>> 0;
};

describe('Proof of work', () => {
  before(async () => {
    await SpvChain.wasmX11Ready();
  });

  describe('compact targets', () => {
    // Vectors from Core's arith_uint256 tests
    [
      [0x00123456, 0n, false, false],
      [0x01003456, 0n, false, false],
      [0x02000056, 0n, false, false],
      [0x03000000, 0n, false, false],
      [0x01123456, 0x12n, false, false],
      [0x01fedcba, 0x7en, true, false],
      [0x02123456, 0x1234n, false, false],
      [0x03123456, 0x123456n, false, false],
      [0x04123456, 0x12345600n, false, false],
      [0x04923456, 0x12345600n, true, false],
      [0x05009234, 0x92340000n, false, false],
      [0x20123456, 0x1234560000000000000000000000000000000000000000000000000000000000n, false, false],
      [0xff123456, 0n, false, true],
    ].forEach(([compact, target, negative, overflow]) => {
      it(`should decode 0x${compact.toString(16)} like Core`, () => {
        const decoded = pow.decodeCompact(compact);
        expect(decoded.negative).to.equal(negative);
        expect(decoded.overflow).to.equal(overflow);
        if (!overflow) {
          expect(decoded.target).to.equal(target);
        }
      });
    });

    [
      [0n, 0],
      [0x12n, 0x01120000],
      [0x80n, 0x02008000],
      [0x1234n, 0x02123400],
      [0x123456n, 0x03123456],
      [0x12345600n, 0x04123456],
      [0x92340000n, 0x05009234],
    ].forEach(([target, compact]) => {
      it(`should encode ${target} as 0x${compact.toString(16)}`, () => {
        expect(pow.encodeCompact(target)).to.equal(compact);
      });
    });
  });

  describe('difficulty rules', () => {
    difficultyCases.forEach((difficultyCase) => {
      const { name, network, height } = difficultyCase;

      it(`should accept the real header for ${name} at ${network} ${height}`, () => {
        const { header, history } = findCase(network, height);

        expect(pow.hasValidDifficulty(header, history, height, network)).to.equal(true);
        expect(consensus.isValidBlockHeader(header, history, network, height)).to.equal(true);
      });

      it(`should compute the mined bits of every header it can in the ${network} ${height} case`, () => {
        const { headers, firstHeight } = caseChain(difficultyCase);
        let checked = 0;

        for (let i = 1; i < headers.length; i += 1) {
          const headerHeight = firstHeight + i;
          const history = headers.slice(0, i);
          const expectedBits = pow.getExpectedBits(
            headers[i],
            history,
            headerHeight,
            network,
            firstHeight,
          );

          if (expectedBits !== null) {
            checked += 1;
            // Early DGW versions mined bits that Core only checks within 50%
            if (network === 'mainnet' && headerHeight >= 34140 && headerHeight <= 68589) {
              expect(pow.hasValidDifficulty(headers[i], history, headerHeight, network))
                .to.equal(true, `height ${headerHeight}`);
            } else {
              expect(headers[i].bits).to.equal(expectedBits, `height ${headerHeight}`);
            }
          }
        }

        expect(checked).to.be.greaterThan(0);
      });
    });

    [
      ['testnet', 1152],
      ['testnet', 4135],
      ['testnet', 4447],
      ['testnet', 1500000],
      ['mainnet', 68590],
      ['mainnet', 2069337],
    ].forEach(([network, height]) => {
      it(`should reject other bits at ${network} ${height}`, () => {
        const { header, history } = findCase(network, height);

        expect(pow.hasValidDifficulty(withBits(header, header.bits + 1), history, height, network))
          .to.equal(false);
      });
    });

    it('should reject the bits a plain average of the DGW window gives at mainnet 2069337', () => {
      const { header, history } = findCase('mainnet', 2069337);

      expect(pow.hasValidDifficulty(withBits(header, 0x193a408b), history, 2069337, 'mainnet'))
        .to.equal(false);
    });

    it('should accept bits within 50% of the expected difficulty up to mainnet 68589', () => {
      const { header, history } = findCase('mainnet', 68589);
      const expectedBits = pow.getExpectedBits(header, history, 68589, 'mainnet');
      const accepts = (bits) => pow.hasValidDifficulty(
        withBits(header, bits),
        history,
        68589,
        'mainnet',
      );

      // Accepted when the block's difficulty is between 2/3 and 2 times the
      // expected one
      expect(accepts(scaleTarget(expectedBits, 1.4))).to.equal(true);
      expect(accepts(scaleTarget(expectedBits, 1.6))).to.equal(false);
      expect(accepts(scaleTarget(expectedBits, 0.55))).to.equal(true);
      expect(accepts(scaleTarget(expectedBits, 0.45))).to.equal(false);
    });

    it('should check difficulty exactly from mainnet 68590', () => {
      const { header, history } = findCase('mainnet', 68590);

      expect(pow.hasValidDifficulty(withBits(header, scaleTarget(header.bits, 1.01)), history, 68590, 'mainnet'))
        .to.equal(false);
    });

    it('should not decide when a rule reads headers from before the chain root', () => {
      const { header, history } = findCase('mainnet', 21000);

      expect(pow.hasValidDifficulty(header, history.slice(-100), 21000, 'mainnet')).to.equal(null);
      expect(pow.hasValidDifficulty(header, history.slice(-1), 21000, 'mainnet')).to.equal(null);
    });

    it('should throw when the history leaves out a header the chain has', () => {
      const { header, history } = findCase('mainnet', 21000);

      expect(() => pow.hasValidDifficulty(header, history.slice(-100), 21000, 'mainnet', 0))
        .to.throw('validation history leaves out');
    });

    it('should check devnet difficulty with Core\'s default devnet rules', () => {
      // DarkGravityWave gives the same bits on a devnet with the default spacing
      const { header, history } = findCase('testnet', 1500000);

      expect(pow.hasValidDifficulty(header, history, 1500000, 'devnet')).to.equal(true);
      expect(pow.hasValidDifficulty(withBits(header, header.bits + 1), history, 1500000, 'devnet'))
        .to.equal(false);
    });

    it('should check devnet difficulty with the settings the devnet was started with', () => {
      const { header, history } = findCase('testnet', 1500000);
      const fastDevnet = { network: 'devnet', powTargetSpacing: 60 };
      const lateDevnet = { network: 'devnet', minimumDifficultyBlocks: 2000000 };

      expect(pow.hasValidDifficulty(header, history, 1500000, fastDevnet)).to.equal(false);
      expect(pow.getExpectedBits(header, history, 1500000, lateDevnet))
        .to.equal(pow.encodeCompact(pow.getNetworkParams('devnet').powLimit));
    });

    it('should accept block spacing and minimum-difficulty settings only for devnets', () => {
      expect(() => pow.getNetworkParams({ network: 'mainnet', powTargetSpacing: 60 }))
        .to.throw('Only devnets');
      expect(() => pow.getNetworkParams({ network: 'devnet', powTargetSpacing: 10 }))
        .to.throw('Unsupported block spacing 10');
      expect(() => new SpvChain('devnet', 100, { powTargetSpacing: 0 })).to.throw();
    });
  });

  describe('proof of work', () => {
    it('should check the hash of a header as it is now, not a memoized one', () => {
      const { header } = findCase('mainnet', 2069337);
      const changed = new BlockHeader(header.toBuffer());
      expect(consensus.isValidBlockHeaderWithoutContext(changed, 'mainnet')).to.equal(true);

      // header.hash is memoized; changing a field doesn't refresh it
      changed.nonce += 1;

      expect(consensus.isValidBlockHeaderWithoutContext(changed, 'mainnet')).to.equal(false);
    });

    it('should accept previous headers in any format a header can be given in', () => {
      const { header, history } = findCase('mainnet', 2069337);
      const rawHistory = history.map((h) => h.toBuffer().toString('hex'));

      expect(consensus.isValidBlockHeader(header, rawHistory, 'mainnet', 2069337)).to.equal(true);
    });
  });

  describe('median time past', () => {
    it('should reject a header not later than the median time of the previous 11', () => {
      const { genesis } = new SpvChain('regtest');
      const history = [genesis];
      while (history.length < 11) {
        history.push(utils.createBlock(history[history.length - 1], 0x207fffff));
      }
      const median = history[5].time;

      const template = utils.createBlock(history[history.length - 1], 0x207fffff);
      let atMedian;
      let nonce = 0;
      do {
        atMedian = withFields(template, { time: median, nonce });
        nonce += 1;
      } while (!utils.validProofOfWork(atMedian));

      expect(pow.getMedianTimePast(history, 11)).to.equal(median);
      expect(consensus.isValidBlockHeader(atMedian, history, 'regtest', 11)).to.equal(false);
    });
  });

  describe('SpvChain', () => {
    it('should keep its own copies of the headers it is given', () => {
      const { history, header } = findCase('testnet', 1152);
      const chain = new SpvChain('testnet', 10000);
      chain.initialize(history[0], 1152 - history.length);
      const given = [...history.slice(1), header].map((h) => new BlockHeader(h.toBuffer()));

      chain.addHeaders(given);
      given[given.length - 1].nonce += 1;

      expect(chain.getTipHeader()).to.not.equal(given[given.length - 1]);
      expect(chain.getTipHeader().nonce).to.equal(header.nonce);
    });

    it('should not be changed through a raw Buffer it was given', () => {
      const { history, header } = findCase('testnet', 1152);
      const chain = new SpvChain('testnet', 10000);
      const rawRoot = history[0].toBuffer();
      chain.initialize(rawRoot, 1152 - history.length);
      const raw = [...history.slice(1), header].map((h) => h.toBuffer());

      chain.addHeaders(raw);
      // prevHash and merkleRoot of a header read from a Buffer are slices of it
      raw[raw.length - 1].fill(0, 4, 68);
      rawRoot.fill(0, 4, 68);

      expect(utils.getHeaderHash(chain.getTipHeader())).to.equal(header.hash);
      expect(() => chain.validate()).to.not.throw();
    });

    it('should validate headers from a checkpoint at their own heights', () => {
      const { history, header } = findCase('testnet', 1152);
      const chain = new SpvChain('testnet', 10000);

      chain.initialize(history[0], 1152 - history.length);
      chain.addHeaders([...history.slice(1), header]);

      expect(chain.getTipHash()).to.equal(header.hash);
      expect(chain.hashesByHeight.get(1152)).to.equal(header.hash);
    });
  });
});
