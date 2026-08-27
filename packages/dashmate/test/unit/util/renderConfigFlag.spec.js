import renderConfigFlag from '../../../src/util/renderConfigFlag.js';

describe('renderConfigFlag', () => {
  // Most nodes have one config, and it is the default. Naming it in every
  // command dashmate prints tells an operator nothing they can act on.
  it('should omit the flag for the config dashmate would act on anyway', () => {
    expect(renderConfigFlag('mainnet', 'mainnet')).to.equal('');
  });

  // The reason the flag exists. An operator running several nodes who pastes a
  // bare command obtains a certificate for, restarts, or bypasses a check on a
  // different one.
  it('should name any other config', () => {
    expect(renderConfigFlag('testnet', 'mainnet')).to.equal(' --config testnet');
  });

  // A collected archive from another machine does not carry the default. Being
  // explicit is only wasteful; being wrong is not recoverable.
  it('should name the config when the default is unknown', () => {
    [undefined, null].forEach((unknown) => {
      expect(renderConfigFlag('mainnet', unknown)).to.equal(' --config mainnet');
    });
  });

  // It carries its own leading space so a command reads correctly either way -
  // no trailing space when it is gone, no double space before what follows.
  it('should leave a command well formed with and without it', () => {
    expect(`dashmate doctor${renderConfigFlag('mainnet', 'mainnet')}`)
      .to.equal('dashmate doctor');
    expect(`dashmate logs${renderConfigFlag('testnet', 'mainnet')} dashmate_helper`)
      .to.equal('dashmate logs --config testnet dashmate_helper');
    expect(`dashmate logs${renderConfigFlag('mainnet', 'mainnet')} dashmate_helper`)
      .to.equal('dashmate logs dashmate_helper');
  });

  // Config names are the operator's to choose, so "no default" has to be
  // distinguished from a config that happens to be called that. Comparing a
  // stringified absent default would omit the flag for this node.
  it('should not mistake a config named like an absent default', () => {
    ['null', 'undefined'].forEach((name) => {
      expect(renderConfigFlag(name, null)).to.equal(` --config ${name}`);
      expect(renderConfigFlag(name, undefined)).to.equal(` --config ${name}`);
    });
  });

  it('should still quote a name a shell would not pass through', () => {
    expect(renderConfigFlag("my node", 'mainnet')).to.equal(" --config 'my node'");
  });
});
