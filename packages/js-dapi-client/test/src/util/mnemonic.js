const should = require('should');
const mnemonic = require('../../../src/utils/mnemonic');

describe('Util - mnemonic', () => {
  it('should generate a valid seed from a mnemonic', async () => {
    const wd = 'inflict about smart zoo ethics ignore retire expand peasant draft sock raven';
    mnemonic.generateSeedFromMnemonic(wd).should.equal('1268d869bdd1e07acf1f0714c07ca093953acb2302281502719cbea5e3d6eaf9187fc98a6b4b0f81beadebb82e08ffd37ab78472464e6570283eafb2d7973534');
  });
  it('should generate a valid mnemonic and seed', async () => {
    const seedObj = mnemonic.generateMnemonicAndSeed();
    seedObj.should.have.property('bits');
    seedObj.bits.should.equal(128);
    seedObj.should.have.property('language');
    seedObj.language.should.equal('english');
    seedObj.should.have.property('seed');
    seedObj.seed.should.have.length(128);
    seedObj.should.have.property('entropy');
    seedObj.entropy.should.have.length(32);
    seedObj.should.have.property('phrase');
    seedObj.should.have.property('passphrase');
    seedObj.passphrase.should.equal('');
  });
});
